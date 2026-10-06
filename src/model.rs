//! The API types every game shares: the game descriptor (lists, capabilities, name and id
//! patterns), list entries, the overview, online players, player profiles and mutation results.
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, fmt};
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

/// What a game offers for its saved player profiles.
#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
pub struct ProfilesSpec {
    pub view: Capability,
    /// Actions on players that are offline, by editing their saved data.
    pub edit_offline: Option<Capability>,
    /// Actions on players that are online, by commands.
    pub edit_live: Option<Capability>,
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
    pub profiles: Option<ProfilesSpec>,
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

/// Where an online player list came from.
#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum OnlineSource {
    Query,
    Rcon,
    Ping,
    Console,
}

#[derive(ToSchema, Serialize, Clone, Debug, PartialEq)]
pub struct Online {
    pub count: u32,
    pub max: u32,
    pub players: Vec<Player>,
    pub source: OnlineSource,
    /// `false`: `count` is right but `players` lacks some names.
    pub complete: bool,
}

#[derive(ToSchema, Serialize)]
pub struct MutationResult {
    pub method: Method,
    pub restart_required: bool,
    /// The server's reply to commands sent over RCON; `None` otherwise.
    pub message: Option<String>,
}

#[derive(ToSchema, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Gamemode {
    Survival,
    Creative,
    Adventure,
    Spectator,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Armor {
    Head,
    Chest,
    Legs,
    Feet,
}

/// A slot of a player's inventory or ender chest, named like Minecraft's command slots
/// (`hotbar.0`, `inventory.26`, `armor.head`, `weapon.offhand`, `enderchest.3`). Ordered for
/// display.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Slot {
    /// 0 to 8.
    Hotbar(u8),
    /// 0 to 26.
    Inventory(u8),
    Armor(Armor),
    Offhand,
    /// 0 to 26.
    EnderChest(u8),
}

pub const HOTBAR_SLOTS: u8 = 9;
pub const INVENTORY_SLOTS: u8 = 27;
pub const ENDER_CHEST_SLOTS: u8 = 27;

/// `text` as a slot index below `count`, written without sign or leading zeros.
fn slot_index(text: &str, count: u8) -> Option<u8> {
    if text.is_empty()
        || !text.bytes().all(|byte| byte.is_ascii_digit())
        || (text.len() > 1 && text.starts_with('0'))
    {
        return None;
    }
    text.parse().ok().filter(|&index| index < count)
}

impl Slot {
    /// The slot with this name; `None` for anything outside the closed set of names.
    pub fn parse(name: &str) -> Option<Self> {
        let (container, index) = name.split_once('.')?;
        Some(match (container, index) {
            ("armor", "head") => Self::Armor(Armor::Head),
            ("armor", "chest") => Self::Armor(Armor::Chest),
            ("armor", "legs") => Self::Armor(Armor::Legs),
            ("armor", "feet") => Self::Armor(Armor::Feet),
            ("weapon", "offhand") => Self::Offhand,
            ("hotbar", index) => Self::Hotbar(slot_index(index, HOTBAR_SLOTS)?),
            ("inventory", index) => Self::Inventory(slot_index(index, INVENTORY_SLOTS)?),
            ("enderchest", index) => Self::EnderChest(slot_index(index, ENDER_CHEST_SLOTS)?),
            _ => return None,
        })
    }
}

impl fmt::Display for Slot {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Hotbar(index) => write!(f, "hotbar.{index}"),
            Self::Inventory(index) => write!(f, "inventory.{index}"),
            Self::Armor(Armor::Head) => f.write_str("armor.head"),
            Self::Armor(Armor::Chest) => f.write_str("armor.chest"),
            Self::Armor(Armor::Legs) => f.write_str("armor.legs"),
            Self::Armor(Armor::Feet) => f.write_str("armor.feet"),
            Self::Offhand => f.write_str("weapon.offhand"),
            Self::EnderChest(index) => write!(f, "enderchest.{index}"),
        }
    }
}

/// A container a profile action empties.
#[derive(ToSchema, Deserialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum Container {
    /// Inventory, hotbar, armor and offhand.
    Inventory,
    EnderChest,
}

#[derive(ToSchema, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct ProfileSummary {
    /// The game's player id (dashed lowercase UUID on Java).
    pub id: String,
    pub name: Option<String>,
    /// RFC 3339, when the game last saved the player.
    pub last_saved: String,
}

#[derive(ToSchema, Serialize)]
pub struct Profiles {
    /// Newest `last_saved` first.
    pub players: Vec<ProfileSummary>,
}

#[derive(ToSchema, Serialize, Clone, Debug, PartialEq)]
pub struct Position {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    /// E.g. `minecraft:overworld`.
    pub dimension: String,
}

#[derive(ToSchema, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Effect {
    pub id: String,
    pub amplifier: i64,
    /// In ticks, -1 for infinite.
    pub duration: i64,
}

#[derive(ToSchema, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Enchantment {
    pub id: String,
    pub level: i64,
}

#[derive(ToSchema, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// A slot name, as profile actions take it.
    pub slot: String,
    /// E.g. `minecraft:diamond_sword`.
    pub id: String,
    pub count: i64,
    /// The custom name as plain text.
    pub name: Option<String>,
    pub enchantments: Vec<Enchantment>,
    pub damage: Option<i64>,
    /// The item as SNBT.
    pub snbt: String,
}

#[derive(ToSchema, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Advancement {
    pub id: String,
    /// The latest criterion's timestamp as the game stored it.
    pub done_at: Option<String>,
}

#[derive(ToSchema, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct Advancements {
    pub done: u32,
    /// Done advancements without recipes, newest first.
    pub items: Vec<Advancement>,
}

/// A player's saved data.
#[derive(ToSchema, Serialize, Clone, Debug, PartialEq)]
pub struct Profile {
    pub id: String,
    pub name: Option<String>,
    /// RFC 3339, when the game last saved the player.
    pub last_saved: String,
    pub data_version: Option<i64>,
    pub gamemode: Option<Gamemode>,
    pub health: Option<f32>,
    pub max_health: Option<f64>,
    pub food: Option<i64>,
    pub saturation: Option<f32>,
    pub xp_level: Option<i64>,
    pub xp_progress: Option<f32>,
    pub xp_total: Option<i64>,
    pub position: Option<Position>,
    /// The respawn point.
    pub spawn: Option<Position>,
    pub effects: Vec<Effect>,
    /// Inventory, hotbar, armor and offhand.
    pub inventory: Vec<Item>,
    pub ender_chest: Vec<Item>,
    /// Category → statistic → value; `None` without a statistics file.
    pub stats: Option<BTreeMap<String, BTreeMap<String, i64>>>,
    pub advancements: Option<Advancements>,
}
