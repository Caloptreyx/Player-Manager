//! The Java Edition query protocol (GameSpy4 over UDP, `enable-query=true`): a handshake for a
//! challenge token, then the full stat with the player names.
use crate::{
    context::Context,
    tunnel::{self, TunnelError},
};
use std::collections::HashMap;
use wings_api::tunnel::MAX_DATAGRAM_SIZE;

const MAGIC: [u8; 2] = [0xFE, 0xFD];
const HANDSHAKE: u8 = 0x09;
const STAT: u8 = 0x00;
/// Servers only keep the low nibble of each byte.
const SESSION: i32 = 0x0102_0304;
/// `splitnum\0`, 0x80, 0x00 before the key/value section.
const STAT_PADDING: usize = 11;
/// `\x01player_\0\0` between the key/value section and the player names.
const PLAYERS_PADDING: usize = 10;

/// The players of a full stat answer.
#[derive(Debug, PartialEq, Eq)]
pub struct FullStat {
    pub count: u32,
    pub max: u32,
    pub names: Vec<String>,
}

fn handshake_request() -> Vec<u8> {
    let mut request = MAGIC.to_vec();
    request.push(HANDSHAKE);
    request.extend_from_slice(&SESSION.to_be_bytes());
    request
}

/// The challenge token of a handshake answer: type, session, the token as ASCII digits, NUL.
fn parse_challenge(answer: &[u8]) -> Result<i32, TunnelError> {
    let digits = match answer {
        [HANDSHAKE, _, _, _, _, rest @ ..] => rest,
        _ => return Err(TunnelError::failed("not a handshake answer")),
    };
    let end = digits
        .iter()
        .position(|&byte| byte == 0)
        .unwrap_or(digits.len());
    std::str::from_utf8(&digits[..end])
        .ok()
        .and_then(|digits| digits.trim().parse().ok())
        .ok_or_else(|| TunnelError::failed("unreadable challenge token"))
}

/// The full stat request: the basic one plus four padding bytes.
fn full_stat_request(challenge: i32) -> Vec<u8> {
    let mut request = MAGIC.to_vec();
    request.push(STAT);
    request.extend_from_slice(&SESSION.to_be_bytes());
    request.extend_from_slice(&challenge.to_be_bytes());
    request.extend_from_slice(&[0; 4]);
    request
}

/// Splits NUL-terminated strings off the front of a byte slice.
struct Strings<'a>(&'a [u8]);

impl<'a> Iterator for Strings<'a> {
    type Item = &'a [u8];

    fn next(&mut self) -> Option<&'a [u8]> {
        if self.0.is_empty() {
            return None;
        }
        let end = self
            .0
            .iter()
            .position(|&byte| byte == 0)
            .unwrap_or(self.0.len());
        let string = &self.0[..end];
        self.0 = self.0.get(end + 1..).unwrap_or_default();
        Some(string)
    }
}

/// A full stat answer: type, session, padding, `key\0value\0` pairs up to an empty key,
/// padding, then player names up to an empty one.
fn parse_full_stat(answer: &[u8]) -> Result<FullStat, TunnelError> {
    let body = match answer {
        [STAT, _, _, _, _, rest @ ..] if rest.len() >= STAT_PADDING => &rest[STAT_PADDING..],
        _ => return Err(TunnelError::failed("not a full stat answer")),
    };
    let mut strings = Strings(body);
    let mut values = HashMap::new();
    loop {
        match strings.next() {
            Some([]) => break,
            Some(key) => {
                let value = strings.next().unwrap_or_default();
                values.insert(key, value);
            }
            None => return Err(TunnelError::failed("the full stat answer ends early")),
        }
    }
    let number = |key: &str| -> Result<u32, TunnelError> {
        values
            .get(key.as_bytes())
            .and_then(|value| std::str::from_utf8(value).ok())
            .and_then(|value| value.trim().parse().ok())
            .ok_or_else(|| TunnelError::failed(format!("the full stat answer has no {key}")))
    };
    let (count, max) = (number("numplayers")?, number("maxplayers")?);

    let names = Strings(strings.0.get(PLAYERS_PADDING..).unwrap_or_default())
        .take_while(|name| !name.is_empty())
        .map(|name| String::from_utf8_lossy(name).into_owned())
        .collect();
    Ok(FullStat { count, max, names })
}

/// Asks the query port for the full stat.
pub async fn full_stat(ctx: &Context<'_>, port: u16) -> Result<FullStat, TunnelError> {
    tunnel::bounded(async {
        let mut socket = ctx.udp(port).await?;
        let mut buffer = vec![0; MAX_DATAGRAM_SIZE];
        socket.send(&handshake_request()).await?;
        let length = socket.recv(&mut buffer).await?;
        let challenge = parse_challenge(&buffer[..length])?;
        socket.send(&full_stat_request(challenge)).await?;
        let length = socket.recv(&mut buffer).await?;
        parse_full_stat(&buffer[..length])
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_requests() {
        assert_eq!(
            handshake_request(),
            [0xFE, 0xFD, 0x09, 0x01, 0x02, 0x03, 0x04]
        );
        assert_eq!(
            full_stat_request(9_513_307),
            [
                0xFE, 0xFD, 0x00, 0x01, 0x02, 0x03, 0x04, 0x00, 0x91, 0x29, 0x5B, 0x00, 0x00, 0x00,
                0x00
            ]
        );
    }

    #[test]
    fn reads_challenge_tokens() {
        assert_eq!(
            parse_challenge(b"\x09\x01\x02\x03\x049513307\x00").unwrap(),
            9_513_307
        );
        assert_eq!(
            parse_challenge(b"\x09\x01\x02\x03\x04-42\x00").unwrap(),
            -42
        );
        assert!(parse_challenge(b"\x00\x01\x02\x03\x0412\x00").is_err());
        assert!(parse_challenge(b"\x09\x01\x02\x03\x04x\x00").is_err());
    }

    #[test]
    fn reads_full_stat_answers() {
        let answer = b"\x00\x01\x02\x03\x04splitnum\x00\x80\x00\
hostname\x00A Minecraft Server\x00gametype\x00SMP\x00game_id\x00MINECRAFT\x00\
version\x001.21.4\x00plugins\x00\x00map\x00world\x00numplayers\x002\x00\
maxplayers\x0020\x00hostport\x0025565\x00hostip\x00172.18.0.2\x00\x00\
\x01player_\x00\x00Notch\x00jeb_\x00\x00";
        assert_eq!(
            parse_full_stat(answer).unwrap(),
            FullStat {
                count: 2,
                max: 20,
                names: vec!["Notch".into(), "jeb_".into()],
            }
        );

        let empty = b"\x00\x01\x02\x03\x04splitnum\x00\x80\x00\
numplayers\x000\x00maxplayers\x0010\x00\x00\x01player_\x00\x00\x00";
        assert_eq!(
            parse_full_stat(empty).unwrap(),
            FullStat {
                count: 0,
                max: 10,
                names: vec![],
            }
        );
        // a basic stat answer has no key/value section
        assert!(
            parse_full_stat(b"\x00\x01\x02\x03\x04A Server\x00SMP\x00world\x002\x0020\x00")
                .is_err()
        );
    }
}
