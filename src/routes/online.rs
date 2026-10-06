pub mod get {
    use crate::{
        context::{Actor, Context, authorize, unsupported},
        model::Online,
    };
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::{GetPermissionManager, GetUser},
        },
        response::{ApiResponse, ApiResponseResult},
    };

    /// The players online, from the first source that names them all (query, RCON, ping,
    /// console), else the fullest partial answer.
    #[utoipa::path(get, path = "/online", responses(
        (status = OK, body = inline(Online)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = CONFLICT, body = ApiError),
        (status = BAD_GATEWAY, body = ApiError),
        (status = GATEWAY_TIMEOUT, body = ApiError),
    ), params(("server" = uuid::Uuid, description = "The server ID")))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        user: GetUser,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
    ) -> ApiResponseResult {
        permissions.has_server_permission("files.read-content")?;

        let ctx = Context::load(&state, &server).await?;
        let game = ctx.require_game()?;
        let descriptor = game.descriptor(&ctx).await?;
        let online = descriptor
            .online
            .ok_or_else(|| unsupported("listing online players"))?;
        authorize(&online, &permissions)?;

        let actor = Actor {
            permissions: &permissions,
            user: user.uuid,
            activity_logger: &activity_logger,
        };
        ApiResponse::new_serialized(game.online(&ctx, &actor).await?).ok()
    }
}
