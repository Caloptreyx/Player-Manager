pub mod put {
    use crate::{
        context::{Actor, Context, authorize, unsupported},
        model::MutationResult,
    };
    use serde::Deserialize;
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::{GetPermissionManager, GetUser},
        },
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        enabled: bool,
    }

    /// Turns the whitelist on or off.
    #[utoipa::path(put, path = "/whitelist", responses(
        (status = OK, body = inline(MutationResult)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = CONFLICT, body = ApiError),
        (status = UNPROCESSABLE_ENTITY, body = ApiError),
    ), params(("server" = uuid::Uuid, description = "The server ID")), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        user: GetUser,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        let ctx = Context::load(&state, &server).await?;
        let game = ctx.require_game()?;
        let descriptor = game.descriptor(&ctx).await?;
        let toggle = descriptor
            .whitelist_toggle
            .ok_or_else(|| unsupported("turning the whitelist on or off"))?;
        authorize(&toggle.access, &permissions)?;

        let actor = Actor {
            permissions: &permissions,
            user: user.uuid,
            activity_logger: &activity_logger,
        };
        let result = game
            .set_whitelist(&ctx, &actor, toggle.method, data.enabled)
            .await?;
        ApiResponse::new_serialized(result).ok()
    }
}
