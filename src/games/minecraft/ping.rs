//! Pings every server answers: Java's Server List Ping (TCP, VarInt-framed packets, a status
//! JSON) and Bedrock's RakNet unconnected ping (UDP). They give the player count; Java adds a
//! sample of names, which servers may hide or fill with text.
use super::java_id;
use crate::{
    context::Context,
    model::{Online, OnlineSource, Player},
    tunnel::{self, TunnelError},
};
use serde_json::Value;
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use wings_api::tunnel::MAX_DATAGRAM_SIZE;

/// The protocol's packet size limit.
const MAX_PACKET: usize = (1 << 21) - 1;
const NIL_UUID: &str = "00000000-0000-0000-0000-000000000000";

const UNCONNECTED_PING: u8 = 0x01;
const UNCONNECTED_PONG: u8 = 0x1C;
const RAKNET_MAGIC: [u8; 16] = [
    0x00, 0xFF, 0xFF, 0x00, 0xFE, 0xFE, 0xFE, 0xFE, 0xFD, 0xFD, 0xFD, 0xFD, 0x12, 0x34, 0x56, 0x78,
];
const CLIENT_GUID: i64 = 0x0050_4D47_5549_4400;

fn write_varint(output: &mut Vec<u8>, value: i32) {
    let mut value = value as u32;
    loop {
        let byte = (value & 0x7F) as u8;
        value >>= 7;
        if value == 0 {
            output.push(byte);
            return;
        }
        output.push(byte | 0x80);
    }
}

/// The VarInt at the start of `bytes` and its length; `None` while its last byte is missing.
fn decode_varint(bytes: &[u8]) -> Result<Option<(i32, usize)>, TunnelError> {
    let mut value: u32 = 0;
    for (index, &byte) in bytes.iter().enumerate() {
        if index == 5 {
            return Err(TunnelError::failed("a VarInt is longer than 5 bytes"));
        }
        value |= u32::from(byte & 0x7F) << (7 * index);
        if byte & 0x80 == 0 {
            return Ok(Some((value as i32, index + 1)));
        }
    }
    Ok(None)
}

async fn read_varint<S: AsyncRead + Unpin>(stream: &mut S) -> Result<i32, TunnelError> {
    let mut bytes = [0; 5];
    for length in 1..=bytes.len() {
        bytes[length - 1] = stream.read_u8().await?;
        if let Some((value, _)) = decode_varint(&bytes[..length])? {
            return Ok(value);
        }
    }
    Err(TunnelError::failed("a VarInt is longer than 5 bytes"))
}

/// The handshake into the status state (protocol -1: any version) and the status request.
fn status_request(host: &str, port: u16) -> Vec<u8> {
    let mut handshake = Vec::new();
    write_varint(&mut handshake, 0x00);
    write_varint(&mut handshake, -1);
    write_varint(&mut handshake, host.len() as i32);
    handshake.extend_from_slice(host.as_bytes());
    handshake.extend_from_slice(&port.to_be_bytes());
    write_varint(&mut handshake, 1);

    let mut request = Vec::with_capacity(handshake.len() + 5);
    write_varint(&mut request, handshake.len() as i32);
    request.extend_from_slice(&handshake);
    request.extend_from_slice(&[0x01, 0x00]);
    request
}

/// A sample entry that names a real player: servers fill the sample with text lines (the nil
/// UUID, `§` formatting, spaces).
fn sample_player(entry: &Value) -> Option<Player> {
    let name = entry.get("name")?.as_str()?;
    if name.is_empty() || name.contains(['§', ' ']) {
        return None;
    }
    let id = entry.get("id").and_then(Value::as_str).and_then(java_id);
    if id.as_deref() == Some(NIL_UUID) {
        return None;
    }
    Some(Player {
        name: name.to_string(),
        id,
    })
}

/// The players of a status JSON; complete when every player online is in the sample.
fn parse_status(json: &[u8]) -> Result<Online, TunnelError> {
    let status: Value = serde_json::from_slice(json)
        .map_err(|err| TunnelError::failed(format!("unreadable status ({err})")))?;
    let players = status
        .get("players")
        .ok_or_else(|| TunnelError::failed("the status has no player count"))?;
    let number = |key: &str| {
        players
            .get(key)
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok())
            .ok_or_else(|| TunnelError::failed(format!("the status has no players.{key}")))
    };
    let (count, max) = (number("online")?, number("max")?);
    let players: Vec<Player> = players
        .get("sample")
        .and_then(Value::as_array)
        .map(|sample| sample.iter().filter_map(sample_player).collect())
        .unwrap_or_default();
    Ok(Online {
        complete: players.len() == count as usize,
        count,
        max,
        players,
        source: OnlineSource::Ping,
    })
}

/// A status response packet (after its length): id 0, then the JSON as a string.
fn parse_response(packet: &[u8]) -> Result<Online, TunnelError> {
    let unreadable = || TunnelError::failed("unreadable status response");
    let (id, id_length) = decode_varint(packet)?.ok_or_else(unreadable)?;
    if id != 0x00 {
        return Err(unreadable());
    }
    let rest = &packet[id_length..];
    let (length, length_length) = decode_varint(rest)?.ok_or_else(unreadable)?;
    let json = usize::try_from(length)
        .ok()
        .and_then(|length| rest.get(length_length..length_length + length))
        .ok_or_else(unreadable)?;
    parse_status(json)
}

async fn status<S: AsyncRead + AsyncWrite + Unpin>(
    stream: &mut S,
    host: &str,
    port: u16,
) -> Result<Online, TunnelError> {
    stream.write_all(&status_request(host, port)).await?;
    stream.flush().await?;
    let length = read_varint(stream).await?;
    let length = usize::try_from(length)
        .ok()
        .filter(|length| (1..=MAX_PACKET).contains(length))
        .ok_or_else(|| TunnelError::failed(format!("status packet length {length}")))?;
    let mut packet = vec![0; length];
    stream.read_exact(&mut packet).await?;
    parse_response(&packet)
}

/// Pings a Java server listening on `port`; `host` is the address players connect with (proxies
/// route on it).
pub async fn java(ctx: &Context<'_>, host: &str, port: u16) -> Result<Online, TunnelError> {
    tunnel::bounded(async {
        let mut stream = ctx.tcp(port).await?;
        status(&mut stream, host, port).await
    })
    .await
}

fn unconnected_ping(time: i64) -> Vec<u8> {
    let mut ping = vec![UNCONNECTED_PING];
    ping.extend_from_slice(&time.to_be_bytes());
    ping.extend_from_slice(&RAKNET_MAGIC);
    ping.extend_from_slice(&CLIENT_GUID.to_be_bytes());
    ping
}

/// An unconnected pong: id, time, server GUID, magic, then a length-prefixed
/// `MCPE;motd;protocol;version;online;max;...` string. It names nobody, so it is complete only
/// when nobody is online.
fn parse_pong(pong: &[u8]) -> Result<Online, TunnelError> {
    let unreadable = || TunnelError::failed("unreadable pong");
    let [UNCONNECTED_PONG, rest @ ..] = pong else {
        return Err(unreadable());
    };
    if rest.get(16..32) != Some(&RAKNET_MAGIC[..]) {
        return Err(unreadable());
    }
    let length = rest
        .get(32..34)
        .map(|length| usize::from(u16::from_be_bytes([length[0], length[1]])))
        .ok_or_else(unreadable)?;
    let text = rest.get(34..34 + length).ok_or_else(unreadable)?;
    let text = String::from_utf8_lossy(text);
    let fields: Vec<&str> = text.split(';').collect();
    let number = |index: usize| -> Result<u32, TunnelError> {
        fields
            .get(index)
            .and_then(|field| field.trim().parse().ok())
            .ok_or_else(unreadable)
    };
    let (count, max) = (number(4)?, number(5)?);
    Ok(Online {
        count,
        max,
        players: Vec::new(),
        source: OnlineSource::Ping,
        complete: count == 0,
    })
}

/// Pings a Bedrock server listening on `port`.
pub async fn bedrock(ctx: &Context<'_>, port: u16) -> Result<Online, TunnelError> {
    tunnel::bounded(async {
        let mut socket = ctx.udp(port).await?;
        socket
            .send(&unconnected_ping(chrono::Utc::now().timestamp_millis()))
            .await?;
        let mut buffer = vec![0; MAX_DATAGRAM_SIZE];
        let length = socket.recv(&mut buffer).await?;
        parse_pong(&buffer[..length])
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    const VARINTS: &[(i32, &[u8])] = &[
        (0, &[0x00]),
        (1, &[0x01]),
        (127, &[0x7F]),
        (128, &[0x80, 0x01]),
        (255, &[0xFF, 0x01]),
        (25565, &[0xDD, 0xC7, 0x01]),
        (2_097_151, &[0xFF, 0xFF, 0x7F]),
        (i32::MAX, &[0xFF, 0xFF, 0xFF, 0xFF, 0x07]),
        (-1, &[0xFF, 0xFF, 0xFF, 0xFF, 0x0F]),
        (i32::MIN, &[0x80, 0x80, 0x80, 0x80, 0x08]),
    ];

    #[test]
    fn varints() {
        for &(value, bytes) in VARINTS {
            let mut output = Vec::new();
            write_varint(&mut output, value);
            assert_eq!(output, bytes, "{value}");
            assert_eq!(
                decode_varint(bytes).unwrap(),
                Some((value, bytes.len())),
                "{value}"
            );
        }
        assert_eq!(decode_varint(&[0x80]).unwrap(), None);
        assert!(decode_varint(&[0x80, 0x80, 0x80, 0x80, 0x80, 0x01]).is_err());
    }

    #[test]
    fn builds_the_status_request() {
        let mut expected = vec![0x13, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x0F, 0x09];
        expected.extend_from_slice(b"localhost");
        expected.extend_from_slice(&[0x63, 0xDD, 0x01, 0x01, 0x00]);
        assert_eq!(status_request("localhost", 25565), expected);
    }

    fn response(json: &str) -> Vec<u8> {
        let mut packet = vec![0x00];
        write_varint(&mut packet, json.len() as i32);
        packet.extend_from_slice(json.as_bytes());
        packet
    }

    #[test]
    fn keeps_real_players_of_the_sample() {
        let json = r#"{"version":{"name":"Paper 1.21.4","protocol":769},"players":{"max":20,"online":5,"sample":[
            {"name":"Notch","id":"069a79f4-44e9-4726-a5be-fca90e38aaf5"},
            {"name":"jeb_","id":"853c80ef3c3749fdaa49938b674adae6"},
            {"name":"§7and 3 more","id":"00000000-0000-0000-0000-000000000000"},
            {"name":"Welcome","id":"00000000-0000-0000-0000-000000000000"},
            {"name":"two words","id":"11111111-1111-1111-1111-111111111111"}
        ]},"description":{"text":"A Minecraft Server"}}"#;
        let online = parse_response(&response(json)).unwrap();
        assert_eq!(
            online,
            Online {
                count: 5,
                max: 20,
                players: vec![
                    Player {
                        name: "Notch".into(),
                        id: Some("069a79f4-44e9-4726-a5be-fca90e38aaf5".into()),
                    },
                    Player {
                        name: "jeb_".into(),
                        id: Some("853c80ef-3c37-49fd-aa49-938b674adae6".into()),
                    },
                ],
                source: OnlineSource::Ping,
                complete: false,
            }
        );
    }

    #[test]
    fn complete_when_the_sample_holds_everyone() {
        let full = parse_response(&response(
            r#"{"players":{"max":10,"online":1,"sample":[{"name":"Alex","id":"ec561538-f3fd-461d-aff5-086b22154bce"}]}}"#,
        ))
        .unwrap();
        assert!(full.complete);
        let empty = parse_response(&response(r#"{"players":{"max":10,"online":0}}"#)).unwrap();
        assert!(empty.complete && empty.players.is_empty());
        let hidden = parse_response(&response(r#"{"players":{"max":10,"online":3}}"#)).unwrap();
        assert!(!hidden.complete);
        assert!(parse_response(&response(r#"{"description":"no players"}"#)).is_err());
        assert!(parse_response(&[0x01, 0x00]).is_err());
    }

    #[tokio::test]
    async fn pings_over_a_stream() {
        let (mut client, mut server) = tokio::io::duplex(4096);
        let (online, ()) = tokio::join!(status(&mut client, "localhost", 25565), async {
            let mut request = vec![0; status_request("localhost", 25565).len()];
            server.read_exact(&mut request).await.unwrap();
            assert_eq!(request, status_request("localhost", 25565));
            let packet = response(r#"{"players":{"max":20,"online":0}}"#);
            let mut framed = Vec::new();
            write_varint(&mut framed, packet.len() as i32);
            framed.extend_from_slice(&packet);
            // the answer arrives in pieces, like tunnel frames
            let (head, tail) = framed.split_at(3);
            server.write_all(head).await.unwrap();
            server.flush().await.unwrap();
            server.write_all(tail).await.unwrap();
        });
        let online = online.unwrap();
        assert_eq!((online.count, online.max, online.complete), (0, 20, true));
    }

    #[test]
    fn builds_unconnected_pings() {
        let ping = unconnected_ping(0x0102_0304_0506_0708);
        assert_eq!(ping.len(), 33);
        assert_eq!(
            ping[..9],
            [0x01, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08]
        );
        assert_eq!(ping[9..25], RAKNET_MAGIC);
        assert_eq!(ping[25..], CLIENT_GUID.to_be_bytes());
    }

    fn pong(text: &str) -> Vec<u8> {
        let mut pong = vec![UNCONNECTED_PONG];
        pong.extend_from_slice(&0x0102_0304_0506_0708_i64.to_be_bytes());
        pong.extend_from_slice(&0x1122_3344_5566_7788_i64.to_be_bytes());
        pong.extend_from_slice(&RAKNET_MAGIC);
        pong.extend_from_slice(&(text.len() as u16).to_be_bytes());
        pong.extend_from_slice(text.as_bytes());
        pong
    }

    #[test]
    fn reads_pongs() {
        let busy = parse_pong(&pong(
            "MCPE;Dedicated Server;748;1.21.40;3;10;13253860892328930865;Bedrock level;Survival;1;19132;19133;",
        ))
        .unwrap();
        assert_eq!(
            busy,
            Online {
                count: 3,
                max: 10,
                players: vec![],
                source: OnlineSource::Ping,
                complete: false,
            }
        );
        let idle = parse_pong(&pong(
            "MCPE;Dedicated Server;748;1.21.40;0;10;1;w;Survival;1;19132;19133;",
        ))
        .unwrap();
        assert!(idle.complete);
        let mut wrong_magic = pong("MCPE;x;1;1;0;10;");
        wrong_magic[20] ^= 0xFF;
        assert!(parse_pong(&wrong_magic).is_err());
        assert!(parse_pong(&pong("MCPE;truncated")).is_err());
    }
}
