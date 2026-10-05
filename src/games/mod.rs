//! The games the extension manages. Routes are game-agnostic: they detect the server's game,
//! enforce the capabilities of its [`Descriptor`] and hand validated input to it.
use crate::{
    context::{Actor, Context, unsupported},
    model::{
        Descriptor, Entry, FileError, Info, ListKind, ListSpec, Method, MutationResult, Online,
        Player,
    },
};
use shared::{models::user::PermissionManager, response::ApiResponse};
use std::{collections::BTreeMap, net::IpAddr};

pub mod minecraft;

/// Every supported game, in tie-break order for detection.
pub static GAMES: &[&dyn Game] = &[
    &minecraft::java::MinecraftJava,
    &minecraft::bedrock::MinecraftBedrock,
];

/// The game of a server: the highest [`Game::detect`] score wins, ties go to the game listed
/// first in [`GAMES`].
pub fn detect(files: &[&str], hints: &[&str]) -> Option<&'static dyn Game> {
    let mut best: Option<(u8, &'static dyn Game)> = None;
    for &game in GAMES {
        if let Some(score) = game.detect(files, hints)
            && best.is_none_or(|(top, _)| score > top)
        {
            best = Some((score, game));
        }
    }
    best.map(|(_, game)| game)
}

/// What a game reports for the overview, besides its descriptor.
#[derive(Default)]
pub struct Contents {
    pub info: Info,
    /// Exactly the kinds of the descriptor's lists.
    pub lists: BTreeMap<ListKind, Vec<Entry>>,
    /// Name suggestions.
    pub known: Vec<Player>,
    pub errors: Vec<FileError>,
}

/// Who a list add is for.
pub enum Subject {
    Player { name: String, id: Option<String> },
    Ip(IpAddr),
}

/// A validated list add: the subject the list's target asks for, a name and an id matching the
/// descriptor's patterns (the id normalized with [`Game::normalize_id`]), and only the fields
/// the [`ListSpec`] accepts (`level` is one of its options, `reason` is trimmed and not blank).
pub struct Add {
    pub subject: Subject,
    pub level: Option<String>,
    pub bypasses_player_limit: Option<bool>,
    pub reason: Option<String>,
}

/// A validated list removal: a name and/or an id (at least one), or an IP address.
pub enum Selector {
    Player {
        name: Option<String>,
        id: Option<String>,
    },
    Ip(IpAddr),
}

/// One server type the extension can manage.
///
/// Adding a game:
/// 1. Write a module (`games/<family>/<game>.rs`, or `games/<game>.rs`) with a unit struct
///    implementing `Game`. [`Game::descriptor`] declares its lists, name/id patterns and
///    capabilities for the current server state; the routes enforce those capabilities and
///    validate request bodies against them before calling the game, so the other methods
///    only do the work. Leave optional features (online players, list edits, whitelist toggle,
///    kick) at their default implementation and `None` in the descriptor when the game lacks
///    them.
/// 2. Add the struct to [`GAMES`] and pick detection scores that rank it correctly against the
///    other games.
/// 3. Optionally add a frontend module in `frontend/src/games/` (display name, avatars,
///    wording); without one the frontend falls back to generic wording.
///
/// If the game needs a list kind that does not exist yet, add it to [`ListKind`].
#[async_trait::async_trait]
pub trait Game: Send + Sync {
    /// Stable id the frontend keys on, e.g. `minecraft_java`.
    fn id(&self) -> &'static str;

    /// Groups games that share code and UI, e.g. `minecraft`.
    fn family(&self) -> &'static str;

    /// How sure it is that the server runs this game, from the names of the files in its root
    /// and `hints` (egg and image names); `None` when it does not look like this game. Scores
    /// compare across games: 40 a file only this game's server has (its binary), 30 files
    /// only it creates, 20 hints and files it shares with few games, 10 files it shares with
    /// related games.
    fn detect(&self, files: &[&str], hints: &[&str]) -> Option<u8>;

    /// A player id that matches the descriptor's id pattern, in the form the game stores.
    fn normalize_id(&self, id: &str) -> String {
        id.to_string()
    }

    /// What the game offers on this server in its current state.
    async fn descriptor(&self, ctx: &Context<'_>) -> Result<Descriptor, ApiResponse>;

    /// Settings, lists, known players and unreadable files. `permissions` only gate optional
    /// reads (the route already checked `files.read-content`).
    async fn overview(
        &self,
        ctx: &Context<'_>,
        permissions: &PermissionManager,
    ) -> Result<Contents, ApiResponse>;

    /// The players online.
    async fn online(&self, _ctx: &Context<'_>, _actor: &Actor<'_>) -> Result<Online, ApiResponse> {
        Err(unsupported("listing online players"))
    }

    /// Adds to the list `spec` describes, the way `method` (from the edit capability) says.
    async fn add(
        &self,
        _ctx: &Context<'_>,
        _actor: &Actor<'_>,
        _method: Method,
        _spec: &ListSpec,
        _add: Add,
    ) -> Result<MutationResult, ApiResponse> {
        Err(unsupported("editing lists"))
    }

    /// Removes from the list `spec` describes, the way `method` says.
    async fn remove(
        &self,
        _ctx: &Context<'_>,
        _actor: &Actor<'_>,
        _method: Method,
        _spec: &ListSpec,
        _selector: Selector,
    ) -> Result<MutationResult, ApiResponse> {
        Err(unsupported("editing lists"))
    }

    /// Turns the whitelist on or off, the way `method` (from the toggle capability) says.
    async fn set_whitelist(
        &self,
        _ctx: &Context<'_>,
        _actor: &Actor<'_>,
        _method: Method,
        _enabled: bool,
    ) -> Result<MutationResult, ApiResponse> {
        Err(unsupported("turning the whitelist on or off"))
    }

    /// Kicks an online player; `name` matches the name pattern, `reason` is validated.
    async fn kick(
        &self,
        _ctx: &Context<'_>,
        _actor: &Actor<'_>,
        _name: &str,
        _reason: Option<&str>,
    ) -> Result<MutationResult, ApiResponse> {
        Err(unsupported("kicking players"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn detected(files: &[&str], hints: &[&str]) -> Option<&'static str> {
        detect(files, hints).map(|game| game.id())
    }

    const JAVA: Option<&str> = Some(minecraft::java::ID);
    const BEDROCK: Option<&str> = Some(minecraft::bedrock::ID);

    #[test]
    fn bedrock_binary_wins() {
        assert_eq!(
            detected(
                &["bedrock_server", "server.properties", "whitelist.json"],
                &[]
            ),
            BEDROCK
        );
        assert_eq!(detected(&["bedrock_server.exe", "eula.txt"], &[]), BEDROCK);
    }

    #[test]
    fn java_files_and_jars() {
        assert_eq!(detected(&["eula.txt"], &[]), JAVA);
        assert_eq!(detected(&["paper-1.21.jar"], &["Bedrock"]), JAVA);
        assert_eq!(detected(&["ops.json", "permissions.json"], &[]), JAVA);
    }

    #[test]
    fn bedrock_lists_and_hints() {
        assert_eq!(
            detected(&["allowlist.json", "server.properties"], &[]),
            BEDROCK
        );
        assert_eq!(
            detected(
                &["server.properties", "whitelist.json"],
                &["Vanilla Bedrock", "ghcr.io/x:debian"]
            ),
            BEDROCK
        );
        assert_eq!(detected(&[], &["ghcr.io/x/bedrock:latest"]), BEDROCK);
    }

    #[test]
    fn shared_files_mean_java() {
        assert_eq!(detected(&["server.properties"], &[]), JAVA);
        assert_eq!(detected(&["whitelist.json"], &["Paper"]), JAVA);
    }

    #[test]
    fn unknown_servers() {
        assert_eq!(detected(&[], &["Rust"]), None);
        assert_eq!(detected(&["config.yml", "start.sh"], &["Velocity"]), None);
    }
}
