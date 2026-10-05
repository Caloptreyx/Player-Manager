//! The API types every game shares: the game descriptor (lists, capabilities, name and id
//! patterns), list entries, the overview and mutation results.
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use utoipa::ToSchema;

#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ServerState {
    Offline,
    Starting,
    Stopping,
    Running,
}

impl From<wings_api::ServerState> for ServerState {
    fn from(state: wings_api::ServerState) -> Self {
        match state {
            wings_api::ServerState::Offline => Self::Offline,
            wings_api::ServerState::Starting => Self::Starting,
            wings_api::ServerState::Stopping => Self::Stopping,
            wings_api::ServerState::Running => Self::Running,
        }
    }
}

#[derive(ToSchema, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Player {
    pub name: String,
    /// The game's player id (dashed lowercase UUID on Java, XUID on Bedrock).
    pub id: Option<String>,
}

/// The player lists games map their own lists onto.
#[derive(ToSchema, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
#[serde(rename_all = "snake_case")]
pub enum ListKind {
    Whitelist,
    Operators,
    Bans,
    IpBans,
}

impl ListKind {
    /// The kind named `kind` in a route path.
    pub fn parse(kind: &str) -> Option<Self> {
        serde_json::from_value(serde_json::Value::from(kind)).ok()
    }
}

/// One row of any list of any game; fields the list does not use are `None`.
#[derive(ToSchema, Serialize, Default, Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// `None` e.g. for Bedrock permission entries no known player has.
    pub name: Option<String>,
    pub id: Option<String>,
    /// IP bans only.
    pub ip: Option<String>,
    /// Operators: a key of the list's level options.
    pub level: Option<String>,
    pub bypasses_player_limit: Option<bool>,
    pub reason: Option<String>,
    pub source: Option<String>,
    pub created: Option<String>,
    pub expires: Option<String>,
}

#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Method {
    Command,
    File,
}

/// Why the current server state forbids an action.
#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Blocked {
    Transition,
    NotRunning,
}

/// An action a game supports and what it takes in the current server state.
#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Capability {
    /// Panel permissions all needed in the current state.
    pub requires: &'static [&'static str],
    /// The UI hides the control from users holding none of these.
    pub visible_with: &'static [&'static str],
    pub blocked: Option<Blocked>,
}

#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct MethodCapability {
    #[serde(flatten)]
    pub access: Capability,
    pub method: Method,
}

#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct KickCapability {
    #[serde(flatten)]
    pub access: Capability,
    /// Whether a kick reason is accepted.
    pub reason: bool,
}

#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum ListTarget {
    Player,
    Ip,
}

/// Whether adds accept a player id in the current mode.
#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum IdField {
    None,
    Optional,
}

#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct Levels {
    pub options: &'static [&'static str],
    pub default: &'static str,
}

/// One list of a game and what adding to it accepts in the current mode.
#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ListSpec {
    pub kind: ListKind,
    pub target: ListTarget,
    pub id: IdField,
    /// Adds accept a reason.
    pub reason: bool,
    /// `None`: no level choice in the current mode.
    pub levels: Option<Levels>,
    /// Adds accept `bypasses_player_limit`.
    pub bypasses_player_limit: bool,
}

#[derive(ToSchema, Serialize)]
pub struct PlayerName {
    /// Anchored, valid in both Rust `regex` and JS `RegExp`; the backend validates with it.
    #[serde(serialize_with = "crate::validate::pattern_source")]
    #[schema(value_type = String)]
    pub pattern: &'static Regex,
}

#[derive(ToSchema, Serialize)]
pub struct PlayerId {
    /// `uuid`, `xuid`...
    pub kind: &'static str,
    #[serde(serialize_with = "crate::validate::pattern_source")]
    #[schema(value_type = String)]
    pub pattern: &'static Regex,
}

/// What a game offers on a server in its current state.
#[derive(ToSchema, Serialize)]
pub struct Descriptor {
    pub player_name: PlayerName,
    pub player_id: PlayerId,
    /// In display order.
    pub lists: Vec<ListSpec>,
    /// List adds and removes.
    pub edit: Option<MethodCapability>,
    pub whitelist_toggle: Option<MethodCapability>,
    pub online: Option<Capability>,
    pub kick: Option<KickCapability>,
}

/// The detected game as the overview reports it: its identity from [`crate::games::Game`]
/// plus its descriptor.
#[derive(ToSchema, Serialize)]
pub struct GameInfo {
    /// Stable id the frontend keys on, e.g. `minecraft_java`.
    pub id: &'static str,
    /// Groups games that share code and UI, e.g. `minecraft`.
    pub family: &'static str,
    #[serde(flatten)]
    pub descriptor: Descriptor,
}

#[derive(ToSchema, Serialize, Default)]
pub struct Info {
    pub whitelist_enabled: Option<bool>,
    pub max_players: Option<u32>,
    pub online_mode: Option<bool>,
}

/// A list file that exists but does not parse.
#[derive(ToSchema, Serialize)]
pub struct FileError {
    pub file: String,
    pub message: String,
}

#[derive(ToSchema, Serialize)]
pub struct Overview {
    /// `None`: no supported game was detected; everything else is empty.
    pub game: Option<GameInfo>,
    pub state: ServerState,
    pub info: Info,
    /// Exactly the kinds of `game.lists`.
    pub lists: BTreeMap<ListKind, Vec<Entry>>,
    pub known: Vec<Player>,
    pub errors: Vec<FileError>,
}

#[derive(ToSchema, Serialize)]
pub struct Online {
    pub count: u32,
    pub max: u32,
    pub players: Vec<Player>,
}

#[derive(ToSchema, Serialize)]
pub struct MutationResult {
    pub method: Method,
    pub restart_required: bool,
}
