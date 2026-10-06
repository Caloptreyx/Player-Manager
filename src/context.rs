//! What every route needs about a server: its Wings connection, power state, root files and
//! detected game; file and console access logged like the panel's own routes; query tunnels;
//! capability enforcement. Nothing here knows about a particular game.
use crate::{
    console,
    files::Wings,
    games::{self, Game},
    lists,
    model::{Blocked, Capability, ServerState},
    tunnel::TunnelError,
};
use axum::http::StatusCode;
use serde_json::Value;
use shared::{
    State,
    models::{
        server::{Server, ServerActivityLogger},
        user::PermissionManager,
    },
    response::ApiResponse,
};
use std::{collections::HashSet, time::Duration};
use tokio::time::Instant;
use wings_api::{
    DirectoryEntry,
    tunnel::{QueryTcpTunnel, QueryUdpTunnel},
};

/// Console lines read for player suggestions.
const LOG_LINES: u64 = 1000;
/// Console lines compared before and after sending a command that expects an answer.
const TAIL_LINES: u64 = 100;
const POLL_INTERVAL: Duration = Duration::from_millis(250);
/// How long one command may take to answer.
const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(3);

pub fn conflict(message: &str) -> ApiResponse {
    ApiResponse::error(message).with_status(StatusCode::CONFLICT)
}

pub fn player_not_found() -> ApiResponse {
    ApiResponse::error("player not found").with_status(StatusCode::NOT_FOUND)
}

pub fn profile_not_found() -> ApiResponse {
    ApiResponse::error("profile not found").with_status(StatusCode::NOT_FOUND)
}

pub fn unprocessable(message: impl AsRef<str>) -> ApiResponse {
    ApiResponse::error(message).with_status(StatusCode::UNPROCESSABLE_ENTITY)
}

/// A feature the server's game does not have.
pub fn unsupported(feature: &str) -> ApiResponse {
    ApiResponse::error(format!("{feature} is not supported for this game"))
}

pub fn list_not_supported() -> ApiResponse {
    ApiResponse::error("list not supported").with_status(StatusCode::NOT_FOUND)
}

/// Checks a capability of the game's descriptor: 409 when the server state forbids it, the
/// panel's permission error when a required permission is missing.
pub fn authorize(
    capability: &Capability,
    permissions: &PermissionManager,
) -> Result<(), ApiResponse> {
    match capability.blocked {
        Some(Blocked::Transition) => {
            return Err(conflict(
                "the server is starting or stopping, try again in a moment",
            ));
        }
        Some(Blocked::NotRunning) => return Err(conflict("the server is not running")),
        None => {}
    }
    for permission in capability.requires {
        permissions.has_server_permission(permission)?;
    }
    Ok(())
}

/// Who acts: permissions for optional reads, the user files are written as and the activity
/// log of the server.
pub struct Actor<'a> {
    pub permissions: &'a PermissionManager,
    pub user: uuid::Uuid,
    pub activity_logger: &'a ServerActivityLogger,
}

pub struct Context<'a> {
    wings: Wings<'a>,
    pub state: ServerState,
    pub game: Option<&'static dyn Game>,
    /// Names of the files (not directories) in the server root.
    root: HashSet<String>,
    max_size: u64,
}

impl<'a> Context<'a> {
    pub async fn load(state: &State, server: &'a Server) -> Result<Self, ApiResponse> {
        let wings = Wings::connect(state, server).await?;
        let (power_state, entries) = tokio::try_join!(wings.power_state(), wings.list("/"))?;
        let entries = entries.ok_or_else(|| {
            ApiResponse::error("the server's files were not found")
                .with_status(StatusCode::NOT_FOUND)
        })?;
        let root: HashSet<String> = entries
            .into_iter()
            .filter(|entry| !entry.directory)
            .map(|entry| entry.name.to_string())
            .collect();
        let files: Vec<&str> = root.iter().map(String::as_str).collect();
        let game = games::detect(&files, &[server.egg.name.as_str(), server.image.as_str()]);
        let max_size = state
            .settings
            .get_as(|settings| settings.server.max_file_manager_view_size)
            .await?;

        Ok(Self {
            wings,
            state: power_state.into(),
            game,
            root,
            max_size,
        })
    }

    pub fn has(&self, name: &str) -> bool {
        self.root.contains(name)
    }

    pub fn require_game(&self) -> Result<&'static dyn Game, ApiResponse> {
        self.game
            .ok_or_else(|| ApiResponse::error("no supported game was detected on this server"))
    }

    async fn read(&self, name: &str) -> Result<Option<Vec<u8>>, ApiResponse> {
        if !self.has(name) {
            return Ok(None);
        }
        self.read_file(name).await
    }

    /// A file anywhere below the server root (`path` relative to it); `None` when missing.
    pub async fn read_file(&self, path: &str) -> Result<Option<Vec<u8>>, ApiResponse> {
        self.wings.read(&format!("/{path}"), self.max_size).await
    }

    /// The entries of a directory below the server root; `None` when it does not exist.
    pub async fn list(&self, directory: &str) -> Result<Option<Vec<DirectoryEntry>>, ApiResponse> {
        self.wings.list(&format!("/{directory}")).await
    }

    /// The entry of the file `name` in a directory below the server root; `None` when missing.
    pub async fn stat(
        &self,
        directory: &str,
        name: &str,
    ) -> Result<Option<DirectoryEntry>, ApiResponse> {
        self.wings.stat(&format!("/{directory}"), name).await
    }

    /// The root file as text for editing (422 when it is not UTF-8); `None` when missing.
    pub async fn read_text(&self, name: &str) -> Result<Option<String>, ApiResponse> {
        match self.read(name).await? {
            None => Ok(None),
            Some(bytes) => String::from_utf8(bytes)
                .map(Some)
                .map_err(|_| unprocessable(format!("{name} is not UTF-8 text"))),
        }
    }

    /// The root file as text for display; `None` when missing.
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

    /// Writes a root file as the actor and logs it like the panel's file write route.
    pub async fn write(
        &self,
        name: &str,
        content: String,
        actor: &Actor<'_>,
    ) -> Result<(), ApiResponse> {
        self.write_file(name, content.into_bytes(), actor).await
    }

    /// Writes a file anywhere below the server root as the actor and logs it like the panel's
    /// file write route.
    pub async fn write_file(
        &self,
        path: &str,
        content: Vec<u8>,
        actor: &Actor<'_>,
    ) -> Result<(), ApiResponse> {
        if content.len() as u64 > self.max_size {
            return Err(ApiResponse::error(format!("{path} would be too large"))
                .with_status(StatusCode::PAYLOAD_TOO_LARGE));
        }
        let path = format!("/{path}");
        let revision_id = self.wings.write(&path, actor.user, content).await?;
        actor
            .activity_logger
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
        actor: &Actor<'_>,
    ) -> Result<(), ApiResponse> {
        self.write(name, lists::render(entries), actor).await
    }

    pub async fn command(&self, actor: &Actor<'_>, command: &str) -> Result<(), ApiResponse> {
        self.commands(actor, &[command]).await
    }

    /// Sends the commands to the console in one request, each logged like the panel's console
    /// route does.
    pub async fn commands(&self, actor: &Actor<'_>, commands: &[&str]) -> Result<(), ApiResponse> {
        self.wings.commands(actor.activity_logger, commands).await
    }

    /// Sends `command` only when the server is running (e.g. making it reread a file).
    pub async fn command_if_running(
        &self,
        actor: &Actor<'_>,
        command: &str,
    ) -> Result<(), ApiResponse> {
        if self.state == ServerState::Running {
            self.command(actor, command).await?;
        }
        Ok(())
    }

    /// Sends `command` and watches the console tail until `parse` finds its answer among the
    /// lines printed after it; `None` when none came in time.
    pub async fn ask<T>(
        &self,
        actor: &Actor<'_>,
        command: &str,
        parse: fn(&[String]) -> Option<T>,
    ) -> Result<Option<T>, ApiResponse> {
        let baseline = console::split_lines(&self.wings.logs(TAIL_LINES).await?);
        self.command(actor, command).await?;

        let deadline = Instant::now() + ATTEMPT_TIMEOUT;
        while Instant::now() < deadline {
            tokio::time::sleep(POLL_INTERVAL).await;
            let current = console::split_lines(&self.wings.logs(TAIL_LINES).await?);
            if let Some(answer) = parse(console::new_lines(&baseline, &current)) {
                return Ok(Some(answer));
            }
        }
        Ok(None)
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
        match self.wings.logs(LOG_LINES).await {
            Ok(log) => console::split_lines(&log),
            Err(_) => {
                tracing::warn!("could not read the console log for known players");
                Vec::new()
            }
        }
    }

    /// The port of the primary allocation and the address players use with it.
    pub fn address(&self) -> Option<(String, u16)> {
        self.wings.address()
    }

    /// A TCP tunnel to `port` of the server's container; wrap the exchange in
    /// [`crate::tunnel::bounded`].
    pub async fn tcp(&self, port: u16) -> Result<QueryTcpTunnel, TunnelError> {
        self.wings.tcp(port).await
    }

    /// A UDP tunnel to `port` of the server's container.
    pub async fn udp(&self, port: u16) -> Result<QueryUdpTunnel, TunnelError> {
        self.wings.udp(port).await
    }
}
