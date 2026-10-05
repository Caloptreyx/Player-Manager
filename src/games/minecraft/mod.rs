//! Minecraft: Java Edition and Bedrock Edition. Both use `server.properties`, answer the
//! console `list` command and kick with `kick`; the rest lives in [`java`] and [`bedrock`].
use crate::{
    context::{Actor, Context},
    model::{
        Blocked, Capability, KickCapability, Method, MutationResult, Online, Player, ServerState,
    },
    validate,
};
use axum::http::StatusCode;
use regex::Regex;
use shared::response::ApiResponse;
use std::sync::LazyLock;

pub mod bedrock;
mod console;
pub mod java;
mod lookup;
mod properties;

pub const FAMILY: &str = "minecraft";
const PROPERTIES: &str = "server.properties";

// Detection scores (see `Game::detect`): the Bedrock binary decides first, then Java's own
// files and jars, then a "bedrock" hint or Bedrock's list files, then files both share.
const SCORE_BEDROCK_BINARY: u8 = 40;
const SCORE_JAVA_FILES: u8 = 30;
const SCORE_BEDROCK_HINTS: u8 = 20;
const SCORE_SHARED_FILES: u8 = 10;

/// A Java player name, optionally with a Floodgate prefix (`.` or `*`).
pub static JAVA_NAME: LazyLock<Regex> =
    LazyLock::new(|| validate::compile(r"^[.*]?[A-Za-z0-9_]{1,16}$"));
/// A UUID, dashed or not.
pub static UUID: LazyLock<Regex> = LazyLock::new(|| {
    validate::compile(
        r"^(?:[0-9A-Fa-f]{8}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{4}-[0-9A-Fa-f]{12}|[0-9A-Fa-f]{32})$",
    )
});
/// An Xbox gamertag: 1–32 letters, digits and inner spaces.
pub static BEDROCK_NAME: LazyLock<Regex> =
    LazyLock::new(|| validate::compile(r"^[A-Za-z0-9](?:[A-Za-z0-9 ]{0,30}[A-Za-z0-9])?$"));
/// An XUID: 1–20 decimal digits.
pub static XUID: LazyLock<Regex> = LazyLock::new(|| validate::compile(r"^[0-9]{1,20}$"));

/// A UUID, dashed or not, as dashed lowercase.
pub fn java_id(id: &str) -> Option<String> {
    if !UUID.is_match(id) {
        return None;
    }
    uuid::Uuid::parse_str(id)
        .ok()
        .map(|uuid| uuid.hyphenated().to_string())
}

const CONSOLE: &[&str] = &["control.console"];
const CONSOLE_AND_READ: &[&str] = &["control.console", "control.read-console"];
const FILES: &[&str] = &["files.create"];

/// Blocked while the server is starting or stopping.
fn transition(state: ServerState) -> Option<Blocked> {
    matches!(state, ServerState::Starting | ServerState::Stopping).then_some(Blocked::Transition)
}

fn online_capability(state: ServerState) -> Capability {
    Capability {
        requires: CONSOLE_AND_READ,
        visible_with: CONSOLE_AND_READ,
        blocked: match state {
            ServerState::Running => None,
            ServerState::Offline => Some(Blocked::NotRunning),
            ServerState::Starting | ServerState::Stopping => Some(Blocked::Transition),
        },
    }
}

fn kick_capability(state: ServerState) -> KickCapability {
    KickCapability {
        access: Capability {
            requires: CONSOLE,
            visible_with: CONSOLE,
            blocked: (state != ServerState::Running).then_some(Blocked::NotRunning),
        },
        reason: true,
    }
}

/// The players online, from the answer to the first of `commands` that gets one.
async fn online(
    ctx: &Context<'_>,
    actor: &Actor<'_>,
    commands: &[&str],
) -> Result<Online, ApiResponse> {
    for command in commands {
        if let Some(answer) = ctx.ask(actor, command, console::parse_list).await? {
            return Ok(Online {
                count: answer.count,
                max: answer.max,
                players: answer
                    .players
                    .into_iter()
                    .map(|(name, id)| Player { name, id })
                    .collect(),
            });
        }
    }
    Err(
        ApiResponse::error("the server did not answer the list command")
            .with_status(StatusCode::GATEWAY_TIMEOUT),
    )
}

/// The `kick` command; Bedrock needs quotes around gamertags with spaces (Java names have none).
fn kick_command(name: &str, reason: Option<&str>) -> String {
    let target = if name.contains(' ') {
        format!("\"{name}\"")
    } else {
        name.to_string()
    };
    match reason {
        Some(reason) => format!("kick {target} {reason}"),
        None => format!("kick {target}"),
    }
}

async fn kick(
    ctx: &Context<'_>,
    actor: &Actor<'_>,
    name: &str,
    reason: Option<&str>,
) -> Result<MutationResult, ApiResponse> {
    ctx.command(actor, &kick_command(name, reason)).await?;
    Ok(MutationResult {
        method: Method::Command,
        restart_required: false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn java_names() {
        assert!(JAVA_NAME.is_match("Notch"));
        assert!(JAVA_NAME.is_match("a"));
        assert!(JAVA_NAME.is_match("abcdefghijklmnop"));
        assert!(JAVA_NAME.is_match(".Steve_1"));
        assert!(JAVA_NAME.is_match("*Steve"));
        assert!(!JAVA_NAME.is_match(""));
        assert!(!JAVA_NAME.is_match("."));
        assert!(!JAVA_NAME.is_match("abcdefghijklmnopq"));
        assert!(!JAVA_NAME.is_match("..Steve"));
        assert!(!JAVA_NAME.is_match("Ste ve"));
        assert!(!JAVA_NAME.is_match("Steve\nop Hacker"));
        assert!(!JAVA_NAME.is_match("Steve\n"));
        assert!(!JAVA_NAME.is_match("Stéve"));
    }

    #[test]
    fn bedrock_names() {
        assert!(BEDROCK_NAME.is_match("Some Guy 42"));
        assert!(BEDROCK_NAME.is_match("a"));
        assert!(BEDROCK_NAME.is_match(&"a".repeat(32)));
        assert!(!BEDROCK_NAME.is_match(&"a".repeat(33)));
        assert!(!BEDROCK_NAME.is_match(" Guy"));
        assert!(!BEDROCK_NAME.is_match("Guy "));
        assert!(!BEDROCK_NAME.is_match(""));
        assert!(!BEDROCK_NAME.is_match("Guy_1"));
        assert!(!BEDROCK_NAME.is_match("Guy\r"));
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
        assert_eq!(
            java_id("urn:uuid:069a79f4-44e9-4726-a5be-fca90e38aaf5"),
            None
        );
        assert_eq!(java_id("069a79f4-44e94726-a5be-fca90e38aaf5"), None);
        assert_eq!(java_id("069a79f4"), None);
        assert!(XUID.is_match("2535428692371234"));
        assert!(XUID.is_match(&"9".repeat(20)));
        assert!(!XUID.is_match(&"9".repeat(21)));
        assert!(!XUID.is_match(""));
        assert!(!XUID.is_match("12a"));
    }

    #[test]
    fn quotes_names_with_spaces() {
        assert_eq!(
            kick_command("Some Guy", Some("afk")),
            "kick \"Some Guy\" afk"
        );
        assert_eq!(kick_command("Alex", None), "kick Alex");
        assert_eq!(kick_command("Notch", Some("bye now")), "kick Notch bye now");
    }
}
