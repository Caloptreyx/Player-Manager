//! What every route needs about a server: its Wings connection, power state, root files and
//! edition, plus the shared response types, the mutation strategy and id resolution.
use crate::{
    console,
    edition::{self, Edition},
    files::Wings,
    lists::{self, Entry},
    lookup, properties, validate,
};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use shared::{
    State,
    models::{
        server::{Server, ServerActivityLogger},
        user::PermissionManager,
    },
    response::{ApiResponse, ApiResponseResult},
};
use std::collections::{HashMap, HashSet};
use utoipa::ToSchema;

pub const PROPERTIES: &str = "server.properties";
pub const JAVA_WHITELIST: &str = "whitelist.json";
pub const JAVA_OPS: &str = "ops.json";
pub const JAVA_BANS: &str = "banned-players.json";
pub const JAVA_IP_BANS: &str = "banned-ips.json";
pub const JAVA_USERCACHE: &str = "usercache.json";
pub const BEDROCK_ALLOWLIST: &str = "allowlist.json";
pub const BEDROCK_LEGACY_ALLOWLIST: &str = "whitelist.json";
pub const BEDROCK_PERMISSIONS: &str = "permissions.json";

/// The ban reason servers store when none is given.
pub const BAN_REASON: &str = "Banned by an operator.";

/// Console lines searched for Bedrock `Player connected` lines.
const KNOWN_LOG_LINES: u64 = 1000;

#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq)]
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
    /// Dashed lowercase UUID (Java) or XUID (Bedrock).
    pub id: Option<String>,
}

/// A Java op level (1-4) or a Bedrock permission level (`operator`, `member`, `visitor`).
#[derive(ToSchema, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum Level {
    Number(u8),
    Name(String),
}

pub const BEDROCK_LEVELS: [&str; 3] = ["operator", "member", "visitor"];

#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Method {
    Command,
    File,
}

#[derive(ToSchema, Serialize)]
pub struct MutationResult {
    pub method: Method,
    pub restart_required: bool,
}

impl MutationResult {
    pub fn respond(method: Method, restart_required: bool) -> ApiResponseResult {
        ApiResponse::new_serialized(Self {
            method,
            restart_required,
        })
        .ok()
    }
}

pub fn conflict(message: &str) -> ApiResponse {
    ApiResponse::error(message).with_status(StatusCode::CONFLICT)
}

pub fn player_not_found() -> ApiResponse {
    ApiResponse::error("player not found").with_status(StatusCode::NOT_FOUND)
}

pub fn unprocessable(message: impl AsRef<str>) -> ApiResponse {
    ApiResponse::error(message).with_status(StatusCode::UNPROCESSABLE_ENTITY)
}

pub fn java_only() -> ApiResponse {
    ApiResponse::error("Bedrock servers have no ban list")
}

/// Whether the entry's `uuid` is `uuid` (dashed lowercase), however it is written.
pub fn uuid_is(entry: &Entry, uuid: &str) -> bool {
    lists::string(entry, "uuid")
        .and_then(validate::java_id)
        .is_some_and(|found| found == uuid)
}

pub fn xuid_is(entry: &Entry, xuid: &str) -> bool {
    lists::text(entry, "xuid").is_some_and(|found| found == xuid)
}

/// `created` of a new ban entry, in the server's `yyyy-MM-dd HH:mm:ss Z` format.
pub fn ban_created() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S +0000")
        .to_string()
}

/// Players of a Java `usercache.json`.
pub fn usercache_players(entries: &[Value]) -> Vec<Player> {
    lists::objects(entries)
        .filter_map(|entry| {
            Some(Player {
                name: lists::string(entry, "name")?.to_string(),
                id: lists::string(entry, "uuid").and_then(validate::java_id),
            })
        })
        .collect()
}

/// Bedrock players with a known XUID: allowlist entries, then console join lines (newer
/// joins win); one entry per name, ignoring case.
pub fn bedrock_known(allowlist: &[Value], log_lines: &[String]) -> Vec<Player> {
    let from_allowlist = lists::objects(allowlist).filter_map(|entry| {
        let xuid = lists::text(entry, "xuid").filter(|xuid| validate::bedrock_id(xuid))?;
        Some((lists::string(entry, "name")?.to_string(), xuid))
    });
    let mut players: Vec<Player> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for (name, xuid) in from_allowlist.chain(console::connected_players(log_lines)) {
        match index.get(&name.to_ascii_lowercase()) {
            Some(&position) => {
                players[position] = Player {
                    name,
                    id: Some(xuid),
                }
            }
            None => {
                index.insert(name.to_ascii_lowercase(), players.len());
                players.push(Player {
                    name,
                    id: Some(xuid),
                });
            }
        }
    }
    players
}

/// The XUID of `name` among `known` players.
pub fn known_xuid(known: &[Player], name: &str) -> Option<String> {
    known
        .iter()
        .find(|player| player.name.eq_ignore_ascii_case(name))
        .and_then(|player| player.id.clone())
}

/// The name of the known player with XUID `xuid`.
pub fn known_name(known: &[Player], xuid: &str) -> Option<String> {
    known
        .iter()
        .find(|player| player.id.as_deref() == Some(xuid))
        .map(|player| player.name.clone())
}

pub struct Context<'a> {
    pub wings: Wings<'a>,
    pub state: ServerState,
    pub edition: Option<Edition>,
    /// Names of the files (not directories) in the server root.
    root: HashSet<String>,
    max_size: u64,
}

impl<'a> Context<'a> {
    pub async fn load(state: &State, server: &'a Server) -> Result<Self, ApiResponse> {
        let wings = Wings::connect(state, server).await?;
        let (power_state, entries) = tokio::try_join!(wings.power_state(), wings.list("/"))?;
        let root: HashSet<String> = entries
            .into_iter()
            .filter(|entry| !entry.directory)
            .map(|entry| entry.name.to_string())
            .collect();
        let files: Vec<&str> = root.iter().map(String::as_str).collect();
        let edition = edition::detect(&files, &[server.egg.name.as_str(), server.image.as_str()]);
        let max_size = state
            .settings
            .get_as(|settings| settings.server.max_file_manager_view_size)
            .await?;

        Ok(Self {
            wings,
            state: power_state.into(),
            edition,
            root,
            max_size,
        })
    }

    pub fn has(&self, name: &str) -> bool {
        self.root.contains(name)
    }

    pub fn require_edition(&self) -> Result<Edition, ApiResponse> {
        self.edition
            .ok_or_else(|| ApiResponse::error("this server was not detected as a Minecraft server"))
    }

    /// 409 while the server is starting or stopping.
    pub fn require_settled(&self) -> Result<(), ApiResponse> {
        match self.state {
            ServerState::Starting | ServerState::Stopping => Err(conflict(
                "the server is starting or stopping, try again in a moment",
            )),
            ServerState::Offline | ServerState::Running => Ok(()),
        }
    }

    pub fn require_running(&self) -> Result<(), ApiResponse> {
        if self.state == ServerState::Running {
            Ok(())
        } else {
            Err(conflict("the server is not running"))
        }
    }

    /// How a list mutation is applied, after checking the permissions it needs: commands on
    /// a running Java server, file edits otherwise (plus a reload command on a running
    /// Bedrock server).
    pub fn plan(&self, permissions: &PermissionManager) -> Result<(Edition, Method), ApiResponse> {
        let edition = self.require_edition()?;
        self.require_settled()?;
        let running = self.state == ServerState::Running;
        let method = match edition {
            Edition::Java if running => {
                permissions.has_server_permission("control.console")?;
                Method::Command
            }
            Edition::Java => {
                permissions.has_server_permission("files.create")?;
                Method::File
            }
            Edition::Bedrock => {
                permissions.has_server_permission("files.create")?;
                if running {
                    permissions.has_server_permission("control.console")?;
                }
                Method::File
            }
        };
        Ok((edition, method))
    }

    async fn read(&self, name: &str) -> Result<Option<Vec<u8>>, ApiResponse> {
        if !self.has(name) {
            return Ok(None);
        }
        self.wings.read(&format!("/{name}"), self.max_size).await
    }

    /// The file as text for editing (422 when it is not UTF-8); `None` when missing.
    pub async fn read_text(&self, name: &str) -> Result<Option<String>, ApiResponse> {
        match self.read(name).await? {
            None => Ok(None),
            Some(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| unprocessable(format!("{name} is not UTF-8 text"))),
        }
    }

    /// The file as text for display; `None` when missing.
    pub async fn read_text_lossy(&self, name: &str) -> Result<Option<String>, ApiResponse> {
        Ok(self
            .read(name)
            .await?
            .map(|bytes| String::from_utf8_lossy(&bytes).into_owned()))
    }

    /// The entries of a list file, or why it does not parse; `None` when missing.
    pub async fn read_list(
        &self,
        name: &str,
    ) -> Result<Option<Result<Vec<Value>, String>>, ApiResponse> {
        Ok(self
            .read(name)
            .await?
            .map(|bytes| match String::from_utf8(bytes) {
                Ok(text) => lists::parse(&text),
                Err(_) => Err("the file is not UTF-8 text".to_string()),
            }))
    }

    /// The entries of a list file for editing: empty when missing, 422 when it does not parse.
    pub async fn load_list(&self, name: &str) -> Result<Vec<Value>, ApiResponse> {
        match self.read_list(name).await? {
            None => Ok(Vec::new()),
            Some(Ok(entries)) => Ok(entries),
            Some(Err(message)) => Err(unprocessable(format!(
                "{name} could not be parsed: {message}"
            ))),
        }
    }

    /// Writes a root file as `user` and logs it like the panel's file write route.
    pub async fn write(
        &self,
        name: &str,
        content: String,
        user: uuid::Uuid,
        activity_logger: &ServerActivityLogger,
    ) -> Result<(), ApiResponse> {
        if content.len() as u64 > self.max_size {
            return Err(ApiResponse::error(format!("{name} would be too large"))
                .with_status(StatusCode::PAYLOAD_TOO_LARGE));
        }
        let path = format!("/{name}");
        let revision_id = self.wings.write(&path, user, content.into_bytes()).await?;
        activity_logger
            .log(
                "server:file.write",
                serde_json::json!({ "file": path, "revision_id": revision_id }),
            )
            .await;
        Ok(())
    }

    pub async fn write_list(
        &self,
        name: &str,
        entries: &[Value],
        user: uuid::Uuid,
        activity_logger: &ServerActivityLogger,
    ) -> Result<(), ApiResponse> {
        self.write(name, lists::render(entries), user, activity_logger)
            .await
    }

    pub async fn command(
        &self,
        activity_logger: &ServerActivityLogger,
        command: &str,
    ) -> Result<(), ApiResponse> {
        self.wings.command(activity_logger, command).await
    }

    /// Sends `command` when the server is running (Bedrock list reloads).
    pub async fn reload(
        &self,
        activity_logger: &ServerActivityLogger,
        command: &str,
    ) -> Result<(), ApiResponse> {
        if self.state == ServerState::Running {
            self.command(activity_logger, command).await?;
        }
        Ok(())
    }

    /// Bedrock's allowlist file: `allowlist.json`, or the legacy `whitelist.json` when only
    /// it exists.
    pub fn bedrock_allowlist(&self) -> &'static str {
        if !self.has(BEDROCK_ALLOWLIST) && self.has(BEDROCK_LEGACY_ALLOWLIST) {
            BEDROCK_LEGACY_ALLOWLIST
        } else {
            BEDROCK_ALLOWLIST
        }
    }

    /// The command that makes a running Bedrock server reread its allowlist file.
    pub fn bedrock_allowlist_reload(&self) -> &'static str {
        if self.bedrock_allowlist() == BEDROCK_LEGACY_ALLOWLIST {
            "whitelist reload"
        } else {
            "allowlist reload"
        }
    }

    /// The console tail as clean lines, only for users who may read the console; failures
    /// only cost the suggestions they would have given.
    pub async fn log_lines(&self, permissions: &PermissionManager) -> Vec<String> {
        if permissions
            .has_server_permission("control.read-console")
            .is_err()
        {
            return Vec::new();
        }
        match self.wings.logs(KNOWN_LOG_LINES).await {
            Ok(log) => console::split_lines(&log),
            Err(_) => {
                tracing::warn!("could not read the console log for known players");
                Vec::new()
            }
        }
    }

    /// Known Bedrock players from the allowlist file (when it parses) and the console.
    pub async fn bedrock_known_players(
        &self,
        permissions: &PermissionManager,
    ) -> Result<Vec<Player>, ApiResponse> {
        let allowlist = self
            .read_list(self.bedrock_allowlist())
            .await?
            .and_then(Result::ok)
            .unwrap_or_default();
        Ok(bedrock_known(
            &allowlist,
            &self.log_lines(permissions).await,
        ))
    }

    /// `(name, dashed uuid)` of a Java player for file edits: the given id, `usercache.json`,
    /// GeyserMC for Floodgate names, then Mojang, or the offline UUID when `online-mode` is
    /// false.
    pub async fn resolve_java(
        &self,
        name: &str,
        id: Option<String>,
    ) -> Result<(String, String), ApiResponse> {
        if let Some(id) = id {
            return Ok((name.to_string(), id));
        }
        if let Some(Ok(cache)) = self.read_list(JAVA_USERCACHE).await?
            && let Some(player) = usercache_players(&cache)
                .into_iter()
                .find(|player| player.name.eq_ignore_ascii_case(name))
            && let Some(uuid) = player.id
        {
            return Ok((player.name, uuid));
        }
        if let Some(gamertag) = validate::floodgate_gamertag(name) {
            // Floodgate writes the spaces of gamertags as underscores
            let xuid = lookup::geyser_xuid(&gamertag.replace('_', " "))
                .await?
                .ok_or_else(player_not_found)?;
            let uuid = lookup::floodgate_uuid(&xuid).ok_or_else(player_not_found)?;
            return Ok((name.to_string(), uuid));
        }
        let online_mode = self
            .read_text_lossy(PROPERTIES)
            .await?
            .and_then(|content| properties::get_bool(&content, "online-mode"));
        if online_mode == Some(false) {
            return Ok((name.to_string(), lookup::offline_uuid(name)));
        }
        let (uuid, canonical) = lookup::mojang_profile(name)
            .await?
            .ok_or_else(player_not_found)?;
        Ok((canonical, uuid))
    }

    /// The XUID of a Bedrock player: the given id, known players, then GeyserMC.
    pub async fn resolve_xuid(
        &self,
        name: &str,
        id: Option<String>,
        known: &[Player],
    ) -> Result<String, ApiResponse> {
        if let Some(id) = id {
            return Ok(id);
        }
        if let Some(xuid) = known_xuid(known, name) {
            return Ok(xuid);
        }
        lookup::geyser_xuid(name).await?.ok_or_else(|| {
            unprocessable("XUID unknown, let the player join once or enter the XUID")
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_bedrock_players_prefer_newer_joins() {
        let allowlist = lists::parse(
            r#"[{"name":"Alex","xuid":"1"},{"name":"NoXuid"},{"name":"Some Guy","xuid":2}]"#,
        )
        .unwrap();
        let log = console::split_lines(
            "[2026-01-01 12:00:00:000 INFO] Player connected: alex, xuid: 3\nPlayer connected: Steve, xuid: 4",
        );
        let known = bedrock_known(&allowlist, &log);
        let player = |name: &str, id: &str| Player {
            name: name.into(),
            id: Some(id.into()),
        };
        assert_eq!(
            known,
            vec![
                player("alex", "3"),
                player("Some Guy", "2"),
                player("Steve", "4")
            ]
        );
        assert_eq!(known_xuid(&known, "ALEX").as_deref(), Some("3"));
        assert_eq!(known_xuid(&known, "NoXuid"), None);
        assert_eq!(known_name(&known, "2").as_deref(), Some("Some Guy"));
    }
}
