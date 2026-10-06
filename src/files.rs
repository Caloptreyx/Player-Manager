//! Server access through Wings (files, power state, console, query tunnels), with the same
//! checks and error mapping as the panel's own routes.
use crate::{tunnel::TunnelError, validate};
use axum::http::StatusCode;
use shared::{
    ApiError, State,
    models::server::{Server, ServerActivityLogger},
    response::ApiResponse,
};
use tokio::io::AsyncReadExt;
use wings_api::{
    client::{ApiHttpError, AsyncRequestReader, WingsClient},
    tunnel::{QueryTcpTunnel, QueryUdpTunnel},
};

const LIST_PER_PAGE: u64 = 100;
const LIST_MAX_PAGES: u64 = 50;

fn wings_status(status: StatusCode, err: wings_api::ApiError) -> ApiResponse {
    ApiResponse::new_serialized(ApiError::new_wings_value(err)).with_status(status)
}

fn tunnel_error(err: ApiHttpError) -> TunnelError {
    TunnelError::failed(match err {
        ApiHttpError::Http(status, err) => {
            format!("Wings could not open a tunnel ({status}): {}", err.error)
        }
        ApiHttpError::WebSocket(err) => format!("Wings could not open a tunnel: {err}"),
        other => format!("Wings could not open a tunnel: {other:?}"),
    })
}

pub struct Wings<'a> {
    client: WingsClient,
    server: &'a Server,
}

impl<'a> Wings<'a> {
    pub async fn connect(state: &State, server: &'a Server) -> Result<Self, ApiResponse> {
        let client = server
            .node
            .fetch_cached(&state.database)
            .await?
            .api_client(&state.database)
            .await?
            .ignoring(server.subuser_ignored_files.clone().unwrap_or_default());
        Ok(Self { client, server })
    }

    fn ignored(&self) -> Option<Vec<compact_str::CompactString>> {
        self.server.subuser_ignored_files.clone()
    }

    /// Every entry of the directory (up to [`LIST_MAX_PAGES`] pages); `None` when it does not
    /// exist.
    pub async fn list(
        &self,
        directory: &str,
    ) -> Result<Option<Vec<wings_api::DirectoryEntry>>, ApiResponse> {
        let mut query = wings_api::servers_server_files_list::get::Query {
            directory: Some(directory.into()),
            ignored: self.ignored(),
            per_page: Some(LIST_PER_PAGE),
            ..Default::default()
        };
        let mut entries = Vec::new();
        for page in 1..=LIST_MAX_PAGES {
            query.page = Some(page);
            let response = match self
                .client
                .get_servers_server_files_list(self.server.uuid, &query)
                .await
            {
                Ok(response) => response,
                Err(ApiHttpError::Http(StatusCode::NOT_FOUND, _)) => return Ok(None),
                Err(err) => return Err(err.into()),
            };
            let received = response.entries.len() as u64;
            entries.extend(response.entries);
            if received < LIST_PER_PAGE || entries.len() as u64 >= response.total {
                break;
            }
        }
        Ok(Some(entries))
    }

    /// The entry of the file `name` in `directory`; `None` when either does not exist.
    pub async fn stat(
        &self,
        directory: &str,
        name: &str,
    ) -> Result<Option<wings_api::DirectoryEntry>, ApiResponse> {
        let request_body = wings_api::servers_server_files_stat::post::RequestBody {
            root: directory.into(),
            files: vec![name.into()],
        };
        match self
            .client
            .post_servers_server_files_stat(self.server.uuid, &request_body)
            .await
        {
            Ok(response) => Ok(response
                .entries
                .into_iter()
                .find(|entry| entry.name == name && !entry.directory)),
            Err(ApiHttpError::Http(StatusCode::NOT_FOUND, _)) => Ok(None),
            Err(err) => Err(err.into()),
        }
    }

    /// The file's bytes, at most `max_size` of them (413 beyond); `None` when it does not exist.
    pub async fn read(&self, path: &str, max_size: u64) -> Result<Option<Vec<u8>>, ApiResponse> {
        let reader = match self
            .client
            .get_servers_server_files_contents(
                self.server.uuid,
                &wings_api::servers_server_files_contents::get::Query {
                    file: Some(path.into()),
                    max_size: Some(max_size),
                    ignored: self.ignored(),
                    ..Default::default()
                },
            )
            .await
        {
            Ok(reader) => reader,
            Err(ApiHttpError::Http(StatusCode::NOT_FOUND, _)) => return Ok(None),
            Err(ApiHttpError::Http(StatusCode::PAYLOAD_TOO_LARGE, err)) => {
                return Err(wings_status(StatusCode::PAYLOAD_TOO_LARGE, err));
            }
            Err(err) => return Err(err.into()),
        };

        let mut data = Vec::new();
        reader
            .take(max_size.saturating_add(1))
            .read_to_end(&mut data)
            .await?;
        if data.len() as u64 > max_size {
            return Err(ApiResponse::error(format!("{path} is too large"))
                .with_status(StatusCode::PAYLOAD_TOO_LARGE));
        }
        Ok(Some(data))
    }

    /// Writes the file as `user`; the Wings revision id of the new content.
    pub async fn write(
        &self,
        path: &str,
        user: uuid::Uuid,
        data: Vec<u8>,
    ) -> Result<Option<i64>, ApiResponse> {
        match self
            .client
            .post_servers_server_files_write(
                self.server.uuid,
                AsyncRequestReader::new(std::io::Cursor::new(data)),
                &wings_api::servers_server_files_write::post::Query {
                    file: Some(path.into()),
                    user: Some(user),
                    ignored: self.ignored(),
                    ..Default::default()
                },
            )
            .await
        {
            Ok(response) => Ok(response.revision_id),
            Err(ApiHttpError::Http(
                status @ (StatusCode::NOT_FOUND | StatusCode::EXPECTATION_FAILED),
                err,
            )) => Err(wings_status(status, err)),
            Err(err) => Err(err.into()),
        }
    }

    /// The server's power state.
    pub async fn power_state(&self) -> Result<wings_api::ServerState, ApiResponse> {
        Ok(self
            .client
            .get_servers_server(self.server.uuid)
            .await?
            .state)
    }

    /// Sends console commands in one request and logs each like the panel's console route
    /// does. Commands with control characters (newlines would inject further commands) are
    /// refused.
    pub async fn commands(
        &self,
        activity_logger: &ServerActivityLogger,
        commands: &[&str],
    ) -> Result<(), ApiResponse> {
        for command in commands {
            validate::command(command)?;
        }
        let request_body = wings_api::servers_server_commands::post::RequestBody {
            commands: commands.iter().map(|&command| command.into()).collect(),
        };
        match self
            .client
            .post_servers_server_commands(self.server.uuid, &request_body)
            .await
        {
            Ok(_) => {}
            Err(ApiHttpError::Http(StatusCode::EXPECTATION_FAILED, err)) => {
                return Err(wings_status(StatusCode::CONFLICT, err));
            }
            Err(err) => return Err(err.into()),
        }
        for command in commands {
            activity_logger
                .log(
                    "server:console.command",
                    serde_json::json!({ "command": command }),
                )
                .await;
        }
        Ok(())
    }

    /// The last `lines` lines of the console log, as (lossy) text.
    pub async fn logs(&self, lines: u64) -> Result<String, ApiResponse> {
        let mut reader = self
            .client
            .get_servers_server_logs(
                self.server.uuid,
                &wings_api::servers_server_logs::get::Query {
                    lines: Some(lines),
                    ..Default::default()
                },
            )
            .await?;
        let mut data = Vec::new();
        reader.read_to_end(&mut data).await?;
        Ok(String::from_utf8_lossy(&data).into_owned())
    }

    /// The port of the primary allocation and the address players use with it (its alias, or
    /// its IP).
    pub fn address(&self) -> Option<(String, u16)> {
        let allocation = &self.server.allocation.as_ref()?.allocation;
        let port = u16::try_from(allocation.port).ok()?;
        let host = match allocation.ip_alias.as_deref() {
            Some(alias) => alias.to_string(),
            None => allocation.ip.ip().to_string(),
        };
        Some((host, port))
    }

    /// A TCP tunnel to `port` of the server's container (no timeout of its own).
    pub async fn tcp(&self, port: u16) -> Result<QueryTcpTunnel, TunnelError> {
        self.client
            .open_tunnel_tcp(self.server.uuid, port)
            .await
            .map_err(tunnel_error)
    }

    /// A UDP tunnel to `port` of the server's container (`recv` times out after 5 s).
    pub async fn udp(&self, port: u16) -> Result<QueryUdpTunnel, TunnelError> {
        self.client
            .open_tunnel_udp(self.server.uuid, port)
            .await
            .map_err(tunnel_error)
    }
}
