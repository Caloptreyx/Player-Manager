pub mod get {
    use crate::{
        console::{self, ListAnswer},
        context::{Context, Player, ServerState, conflict, known_xuid},
        edition::Edition,
    };
    use axum::http::StatusCode;
    use serde::Serialize;
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger, ServerActivityLogger},
            user::GetPermissionManager,
        },
        response::{ApiResponse, ApiResponseResult},
    };
    use std::time::Duration;
    use tokio::time::Instant;
    use utoipa::ToSchema;

    /// Console lines compared before and after sending `list`.
    const TAIL_LINES: u64 = 100;
    const POLL_INTERVAL: Duration = Duration::from_millis(250);
    /// How long one `list` command may take to answer.
    const ATTEMPT_TIMEOUT: Duration = Duration::from_secs(3);

    #[derive(ToSchema, Serialize)]
    struct Response {
        count: u32,
        max: u32,
        players: Vec<Player>,
    }

    /// Sends `command` and watches the console tail for the `list` answer printed after it.
    async fn ask(
        ctx: &Context<'_>,
        activity_logger: &ServerActivityLogger,
        command: &str,
    ) -> Result<Option<ListAnswer>, ApiResponse> {
        let baseline = console::split_lines(&ctx.wings.logs(TAIL_LINES).await?);
        ctx.command(activity_logger, command).await?;

        let deadline = Instant::now() + ATTEMPT_TIMEOUT;
        while Instant::now() < deadline {
            tokio::time::sleep(POLL_INTERVAL).await;
            let current = console::split_lines(&ctx.wings.logs(TAIL_LINES).await?);
            if let Some(answer) = console::parse_list(console::new_lines(&baseline, &current)) {
                return Ok(Some(answer));
            }
        }
        Ok(None)
    }

    /// The players online, from the server's answer to `list`.
    #[utoipa::path(get, path = "/online", responses(
        (status = OK, body = inline(Response)),
        (status = UNAUTHORIZED, body = ApiError),
        (status = CONFLICT, body = ApiError),
        (status = GATEWAY_TIMEOUT, body = ApiError),
    ), params(("server" = uuid::Uuid, description = "The server ID")))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
    ) -> ApiResponseResult {
        permissions.has_server_permission("control.console")?;
        permissions.has_server_permission("control.read-console")?;

        let ctx = Context::load(&state, &server).await?;
        if ctx.state != ServerState::Running {
            return Err(conflict("the server is not running"));
        }

        // `list uuids` adds the UUIDs; servers without it answer `list`
        let commands: &[&str] = match ctx.edition {
            Some(Edition::Java) => &["minecraft:list uuids", "list"],
            _ => &["list"],
        };
        for command in commands {
            let Some(answer) = ask(&ctx, &activity_logger, command).await? else {
                continue;
            };
            let mut players: Vec<Player> = answer
                .players
                .into_iter()
                .map(|(name, id)| Player { name, id })
                .collect();
            if ctx.edition == Some(Edition::Bedrock) {
                let known = ctx.bedrock_known_players(&permissions).await?;
                for player in players.iter_mut().filter(|player| player.id.is_none()) {
                    player.id = known_xuid(&known, &player.name);
                }
            }
            return ApiResponse::new_serialized(Response {
                count: answer.count,
                max: answer.max,
                players,
            })
            .ok();
        }

        Err(
            ApiResponse::error("the server did not answer the list command")
                .with_status(StatusCode::GATEWAY_TIMEOUT),
        )
    }
}
