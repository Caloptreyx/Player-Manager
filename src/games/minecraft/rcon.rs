//! Minecraft RCON (Source RCON over TCP): little-endian `length, request id, type, body, 0, 0`
//! packets; a login, then commands whose replies may span several packets.
use crate::tunnel::{self, TunnelError};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

const LOGIN: i32 = 3;
/// Commands, and the login answer.
const COMMAND: i32 = 2;
const RESPONSE: i32 = 0;
/// The request id of a login answer for a wrong password.
const REJECTED: i32 = -1;
/// Vanilla reads a request into a 1460-byte buffer.
const MAX_REQUEST_BODY: usize = 1446;
/// The largest packet accepted (vanilla splits replies into 4096-byte bodies).
const MAX_PACKET: usize = 1 << 20;
/// How long to wait for the rest of a reply after its first packet.
const REST_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Debug, PartialEq, Eq)]
struct Packet {
    id: i32,
    kind: i32,
    body: Vec<u8>,
}

fn encode(id: i32, kind: i32, body: &str) -> Vec<u8> {
    let length = 4 + 4 + body.len() + 2;
    let mut packet = Vec::with_capacity(4 + length);
    packet.extend_from_slice(&(length as i32).to_le_bytes());
    packet.extend_from_slice(&id.to_le_bytes());
    packet.extend_from_slice(&kind.to_le_bytes());
    packet.extend_from_slice(body.as_bytes());
    packet.extend_from_slice(&[0, 0]);
    packet
}

/// A packet from the bytes after its length.
fn decode(bytes: &[u8]) -> Result<Packet, TunnelError> {
    let [a, b, c, d, e, f, g, h, rest @ ..] = bytes else {
        return Err(TunnelError::failed("RCON packet too short"));
    };
    let body = rest.strip_suffix(&[0]).unwrap_or(rest);
    let body = body.strip_suffix(&[0]).unwrap_or(body);
    Ok(Packet {
        id: i32::from_le_bytes([*a, *b, *c, *d]),
        kind: i32::from_le_bytes([*e, *f, *g, *h]),
        body: body.to_vec(),
    })
}

async fn read_packet<S: AsyncRead + Unpin>(stream: &mut S) -> Result<Packet, TunnelError> {
    let length = stream.read_i32_le().await?;
    let length = usize::try_from(length)
        .ok()
        .filter(|length| (8..=MAX_PACKET).contains(length))
        .ok_or_else(|| TunnelError::failed(format!("RCON packet length {length}")))?;
    let mut bytes = vec![0; length];
    stream.read_exact(&mut bytes).await?;
    decode(&bytes)
}

/// A logged-in RCON connection.
pub struct Session<S> {
    stream: S,
    next_id: i32,
}

impl<S: AsyncRead + AsyncWrite + Unpin> Session<S> {
    /// Logs in on `stream` (wrap it in [`tunnel::bounded`]); a rejected password is an error.
    pub async fn login(stream: S, password: &str) -> Result<Self, TunnelError> {
        let mut session = Self { stream, next_id: 1 };
        let id = session.send(LOGIN, password).await?;
        loop {
            let packet = read_packet(&mut session.stream).await?;
            if packet.id == REJECTED {
                return Err(TunnelError::failed("the RCON password was rejected"));
            }
            // some servers send an empty response before the login answer
            if packet.id == id && packet.kind == COMMAND {
                return Ok(session);
            }
        }
    }

    async fn send(&mut self, kind: i32, body: &str) -> Result<i32, TunnelError> {
        if body.len() > MAX_REQUEST_BODY {
            return Err(TunnelError::failed("the command is too long for RCON"));
        }
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1).max(1);
        self.stream.write_all(&encode(id, kind, body)).await?;
        self.stream.flush().await?;
        Ok(id)
    }

    /// Runs `command`; its reply, joined from every packet the server split it into. Waits
    /// [`tunnel::TIMEOUT`] for the reply.
    pub async fn command(&mut self, command: &str) -> Result<String, TunnelError> {
        let id = self.send(COMMAND, command).await?;
        let mut body = tunnel::bounded(async {
            loop {
                let packet = read_packet(&mut self.stream).await?;
                if packet.id == id {
                    return Ok::<_, TunnelError>(packet.body);
                }
            }
        })
        .await?;
        // A request of an unknown type sent now is answered after every packet of the
        // reply. Sent only after the first packet: vanilla drops requests that arrive
        // together. When it gets no answer, the reply so far is all there is.
        if let Ok(end) = self.send(RESPONSE, "").await {
            let _ = tokio::time::timeout(REST_TIMEOUT, async {
                while let Ok(packet) = read_packet(&mut self.stream).await {
                    if packet.id == end {
                        break;
                    }
                    if packet.id == id {
                        body.extend_from_slice(&packet.body);
                    }
                }
            })
            .await;
        }
        Ok(String::from_utf8_lossy(&body).into_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::DuplexStream;

    async fn answer(server: &mut DuplexStream, id: i32, kind: i32, body: &str) {
        server.write_all(&encode(id, kind, body)).await.unwrap();
    }

    #[test]
    fn encodes_packets() {
        assert_eq!(
            encode(1, LOGIN, "password"),
            b"\x12\x00\x00\x00\x01\x00\x00\x00\x03\x00\x00\x00password\x00\x00"
        );
        assert_eq!(
            decode(b"\xff\xff\xff\xff\x02\x00\x00\x00\x00\x00").unwrap(),
            Packet {
                id: -1,
                kind: COMMAND,
                body: vec![],
            }
        );
        assert_eq!(
            decode(b"\x05\x00\x00\x00\x00\x00\x00\x00Done\x00\x00")
                .unwrap()
                .body,
            b"Done"
        );
        assert!(decode(b"\x01\x00\x00\x00").is_err());
    }

    #[tokio::test]
    async fn rejects_wrong_passwords() {
        let (client, mut server) = tokio::io::duplex(4096);
        let (session, ()) = tokio::join!(Session::login(client, "wrong"), async {
            let login = read_packet(&mut server).await.unwrap();
            assert_eq!(
                (login.id, login.kind, login.body),
                (1, LOGIN, b"wrong".to_vec())
            );
            server
                .write_all(b"\x0a\x00\x00\x00\xff\xff\xff\xff\x02\x00\x00\x00\x00\x00")
                .await
                .unwrap();
        });
        assert_eq!(
            session.err(),
            Some(TunnelError::failed("the RCON password was rejected"))
        );
    }

    #[tokio::test]
    async fn joins_replies_split_over_packets() {
        let (client, mut server) = tokio::io::duplex(1 << 16);
        let long = "x".repeat(4096);
        let (reply, ()) = tokio::join!(
            async {
                let mut session = Session::login(client, "secret").await?;
                session.command("list").await
            },
            async {
                let login = read_packet(&mut server).await.unwrap();
                // an empty response first, as some servers send it
                answer(&mut server, login.id, RESPONSE, "").await;
                answer(&mut server, login.id, COMMAND, "").await;
                let command = read_packet(&mut server).await.unwrap();
                assert_eq!((command.kind, command.body), (COMMAND, b"list".to_vec()));
                answer(&mut server, command.id, RESPONSE, &long).await;
                answer(&mut server, command.id, RESPONSE, "tail").await;
                let end = read_packet(&mut server).await.unwrap();
                assert_eq!(end.kind, RESPONSE);
                answer(&mut server, end.id, RESPONSE, "Unknown request 0").await;
            }
        );
        assert_eq!(reply.unwrap(), format!("{long}tail"));
    }
}
