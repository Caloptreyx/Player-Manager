//! Input validation. Everything that ends up in a console command passes through here, so
//! nothing accepted may contain newlines or other control characters.
use regex::Regex;
use serde::Serializer;
use shared::response::ApiResponse;
use std::net::IpAddr;

pub const MAX_REASON_CHARS: usize = 256;

/// Compiles one of the anchored patterns games publish in their descriptor (meant for a
/// `LazyLock`; the patterns are constants, so failing is a bug).
pub fn compile(source: &str) -> Regex {
    Regex::new(source).expect("game patterns are valid regexes")
}

/// Serializes a pattern as its source, the string the frontend compiles with `RegExp`.
pub fn pattern_source<S: Serializer>(pattern: &&Regex, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(pattern.as_str())
}

fn matches(pattern: &Regex, value: &str) -> bool {
    pattern.is_match(value) && !value.chars().any(char::is_control)
}

/// A player name matching the game's `player_name` pattern.
pub fn player_name(pattern: &Regex, name: &str) -> Result<(), ApiResponse> {
    if matches(pattern, name) {
        Ok(())
    } else {
        Err(ApiResponse::error("invalid player name"))
    }
}

/// A player id matching the game's `player_id` pattern.
pub fn player_id(pattern: &Regex, id: &str) -> Result<(), ApiResponse> {
    if matches(pattern, id) {
        Ok(())
    } else {
        Err(ApiResponse::error("invalid player id"))
    }
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
    fn patterns_reject_control_characters() {
        let any = compile(r"^[\s\S]+$");
        assert!(player_name(&any, "Steve").is_ok());
        assert!(player_name(&any, "Steve\nop Hacker").is_err());
        assert!(player_id(&any, "1\r").is_err());
        assert!(player_name(&compile("^a$"), "b").is_err());
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
