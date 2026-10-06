pub mod get {
    use crate::{
        context::{Context, authorize, unsupported},
        model::Profile,
        routes::lists,
    };
    use axum::extract::Path;
    use shared::{
        ApiError, GetState,
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };

    /// A player's saved data: vitals, inventory, ender chest, statistics and advancements.
    #[utoipa::path(get, path = "/profiles/{id}", responses(
        (status = OK, body = inline(Profile)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
        (status = PAYLOAD_TOO_LARGE, body = ApiError),
        (status = UNPROCESSABLE_ENTITY, body = ApiError),
    ), params(
        ("server" = uuid::Uuid, description = "The server ID"),
        ("id" = String, Path, description = "The player id"),
    ))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
        Path((_server, player)): Path<(String, String)>,
    ) -> ApiResponseResult {
        let ctx = Context::load(&state, &server).await?;
        let game = ctx.require_game()?;
        let descriptor = game.descriptor(&ctx).await?;
        let profiles = descriptor
            .profiles
            .ok_or_else(|| unsupported("player profiles"))?;
        authorize(&profiles.view, &permissions)?;
        let id = lists::id(game, &descriptor, &player)?;

        ApiResponse::new_serialized(game.profile(&ctx, &id).await?).ok()
    }
}
