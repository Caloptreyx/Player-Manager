//! Input validation. Everything that ends up in a console command passes through here, so
//! nothing accepted may contain newlines or other control characters.
use crate::edition::Edition;
use shared::response::ApiResponse;
use std::net::IpAddr;

pub const MAX_REASON_CHARS: usize = 256;

/// A Java player name, optionally with a Floodgate prefix (`.` or `*`).
pub fn java_name(name: &str) -> bool {
    let rest = name.strip_prefix(['.', '*']).unwrap_or(name);
    (1..=16).contains(&rest.len())
        && rest
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

/// An Xbox gamertag: 1–32 letters, digits and inner spaces.
pub fn bedrock_name(name: &str) -> bool {
    (1..=32).contains(&name.len())
        && !name.starts_with(' ')
        && !name.ends_with(' ')
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b' ')
}

/// The Floodgate gamertag behind a prefixed Java name (`.Steve` → `Steve`).
pub fn floodgate_gamertag(name: &str) -> Option<&str> {
    name.strip_prefix(['.', '*'])
}

/// A UUID, dashed or not, as dashed lowercase.
pub fn java_id(id: &str) -> Option<String> {
    if id.len() != 32 && id.len() != 36 {
        return None;
    }
    uuid::Uuid::parse_str(id)
        .ok()
        .map(|uuid| uuid.hyphenated().to_string())
}

/// An XUID: 1–20 decimal digits.
pub fn bedrock_id(id: &str) -> bool {
    (1..=20).contains(&id.len()) && id.bytes().all(|byte| byte.is_ascii_digit())
}

pub fn name(edition: Edition, name: &str) -> Result<(), ApiResponse> {
    let valid = match edition {
        Edition::Java => java_name(name),
        Edition::Bedrock => bedrock_name(name),
    };
    if valid {
        Ok(())
    } else {
        Err(ApiResponse::error(match edition {
            Edition::Java => "invalid player name",
            Edition::Bedrock => "invalid gamertag",
        }))
    }
}

/// The normalized id (dashed lowercase UUID or XUID).
pub fn id(edition: Edition, id: &str) -> Result<String, ApiResponse> {
    match edition {
        Edition::Java => java_id(id).ok_or_else(|| ApiResponse::error("invalid UUID")),
        Edition::Bedrock if bedrock_id(id) => Ok(id.to_string()),
        Edition::Bedrock => Err(ApiResponse::error("invalid XUID")),
    }
}

pub fn optional_id(edition: Edition, value: Option<&str>) -> Result<Option<String>, ApiResponse> {
    value.map(|value| id(edition, value)).transpose()
}

/// The trimmed reason, `None` when blank.
pub fn reason(reason: Option<&str>) -> Result<Option<String>, ApiResponse> {
    let Some(reason) = reason.map(str::trim).filter(|reason| !reason.is_empty()) else {
        return Ok(None);
    };
    if reason.chars().count() > MAX_REASON_CHARS {
        return Err(ApiResponse::error(format!(
            "the reason is longer than {MAX_REASON_CHARS} characters"
        )));
    }
    if reason.chars().any(char::is_control) {
        return Err(ApiResponse::error("the reason contains control characters"));
    }
    Ok(Some(reason.to_string()))
}

pub fn ip(ip: &str) -> Result<IpAddr, ApiResponse> {
    ip.trim()
        .parse()
        .map_err(|_| ApiResponse::error("invalid IP address"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_names() {
        assert!(java_name("Notch"));
        assert!(java_name("a"));
        assert!(java_name("abcdefghijklmnop"));
        assert!(java_name(".Steve_1"));
        assert!(java_name("*Steve"));
        assert!(!java_name(""));
        assert!(!java_name("."));
        assert!(!java_name("abcdefghijklmnopq"));
        assert!(!java_name("..Steve"));
        assert!(!java_name("Ste ve"));
        assert!(!java_name("Steve\nop Hacker"));
        assert!(!java_name("Stéve"));
    }

    #[test]
    fn bedrock_names() {
        assert!(bedrock_name("Some Guy 42"));
        assert!(bedrock_name(&"a".repeat(32)));
        assert!(!bedrock_name(&"a".repeat(33)));
        assert!(!bedrock_name(" Guy"));
        assert!(!bedrock_name("Guy "));
        assert!(!bedrock_name(""));
        assert!(!bedrock_name("Guy_1"));
        assert!(!bedrock_name("Guy\r"));
    }

    #[test]
    fn ids() {
        let dashed = "069a79f4-44e9-4726-a5be-fca90e38aaf5";
        assert_eq!(java_id(dashed).as_deref(), Some(dashed));
        assert_eq!(
            java_id("069A79F444E94726A5BEFCA90E38AAF5").as_deref(),
            Some(dashed)
        );
        assert_eq!(java_id("{069a79f4-44e9-4726-a5be-fca90e38aaf5}"), None);
        assert_eq!(java_id("069a79f4"), None);
        assert!(bedrock_id("2535428692371234"));
        assert!(bedrock_id(&"9".repeat(20)));
        assert!(!bedrock_id(&"9".repeat(21)));
        assert!(!bedrock_id(""));
        assert!(!bedrock_id("12a"));
    }

    #[test]
    fn reasons() {
        assert_eq!(reason(None).ok(), Some(None));
        assert_eq!(reason(Some("   ")).ok(), Some(None));
        assert_eq!(
            reason(Some(" griefing ")).ok(),
            Some(Some("griefing".into()))
        );
        assert!(reason(Some("é".repeat(MAX_REASON_CHARS).as_str())).is_ok());
        assert!(reason(Some("é".repeat(MAX_REASON_CHARS + 1).as_str())).is_err());
        assert!(reason(Some("x\nop Hacker")).is_err());
        assert!(reason(Some("x\u{1b}[31m")).is_err());
    }

    #[test]
    fn ips() {
        assert!(ip("127.0.0.1").is_ok());
        assert!(ip("::1").is_ok());
        assert!(ip("256.0.0.1").is_err());
        assert!(ip("1.2.3.4\nstop").is_err());
    }
}
