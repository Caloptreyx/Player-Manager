pub mod get {
    use crate::{
        context::{Context, authorize, unsupported},
        model::Profiles,
    };
    use shared::{
        ApiError, GetState,
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };

    /// The players the game saved data for, newest first.
    #[utoipa::path(get, path = "/profiles", responses(
        (status = OK, body = inline(Profiles)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
    ), params(("server" = uuid::Uuid, description = "The server ID")))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
    ) -> ApiResponseResult {
        let ctx = Context::load(&state, &server).await?;
        let game = ctx.require_game()?;
        let descriptor = game.descriptor(&ctx).await?;
        let profiles = descriptor
            .profiles
            .ok_or_else(|| unsupported("player profiles"))?;
        authorize(&profiles.view, &permissions)?;

        ApiResponse::new_serialized(Profiles {
            players: game.profiles(&ctx).await?,
        })
        .ok()
    }
}
