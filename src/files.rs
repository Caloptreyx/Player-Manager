//! Server access through Wings (files, power state, console), with the same checks and error
//! mapping as the panel's own routes.
use axum::http::StatusCode;
use shared::{
    ApiError, State,
    models::server::{Server, ServerActivityLogger},
    response::ApiResponse,
};
use tokio::io::AsyncReadExt;
use wings_api::client::{ApiHttpError, AsyncRequestReader, WingsClient};

const LIST_PER_PAGE: u64 = 100;
const LIST_MAX_PAGES: u64 = 50;

fn wings_status(status: StatusCode, err: wings_api::ApiError) -> ApiResponse {
    ApiResponse::new_serialized(ApiError::new_wings_value(err)).with_status(status)
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
            .await?;
        Ok(Self { client, server })
    }

    fn ignored(&self) -> Option<Vec<compact_str::CompactString>> {
        self.server.subuser_ignored_files.clone()
    }

    /// Every entry of the directory (up to [`LIST_MAX_PAGES`] pages).
    pub async fn list(
        &self,
        directory: &str,
    ) -> Result<Vec<wings_api::DirectoryEntry>, ApiResponse> {
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
                Err(ApiHttpError::Http(StatusCode::NOT_FOUND, err)) => {
                    return Err(wings_status(StatusCode::NOT_FOUND, err));
                }
                Err(err) => return Err(err.into()),
            };
            let received = response.entries.len() as u64;
            entries.extend(response.entries);
            if received < LIST_PER_PAGE || entries.len() as u64 >= response.total {
                break;
            }
        }
        Ok(entries)
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

    /// Sends one console command and logs it like the panel's console route does. Commands
    /// with control characters (newlines would inject further commands) are refused.
    pub async fn command(
        &self,
        activity_logger: &ServerActivityLogger,
        command: &str,
    ) -> Result<(), ApiResponse> {
        if command.is_empty() || command.chars().any(char::is_control) {
            return Err(ApiResponse::error("invalid command"));
        }
        let request_body = wings_api::servers_server_commands::post::RequestBody {
            commands: vec![command.into()],
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
        activity_logger
            .log(
                "server:console.command",
                serde_json::json!({ "command": command }),
            )
            .await;
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
}
