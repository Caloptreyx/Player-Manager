pub mod post {
    use crate::{
        context::{Actor, Context, authorize, unsupported},
        model::MutationResult,
        validate,
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
        name: String,
        #[serde(default)]
        reason: Option<String>,
    }

    /// Kicks an online player.
    #[utoipa::path(post, path = "/kick", responses(
        (status = OK, body = inline(MutationResult)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = CONFLICT, body = ApiError),
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
        let kick = descriptor
            .kick
            .ok_or_else(|| unsupported("kicking players"))?;
        authorize(&kick.access, &permissions)?;
        validate::player_name(descriptor.player_name.pattern, &data.name)?;
        let reason = validate::reason(data.reason.as_deref())?;
        if reason.is_some() && !kick.reason {
            return Err(ApiResponse::error("a kick reason is not accepted"));
        }

        let actor = Actor {
            permissions: &permissions,
            user: user.uuid,
            activity_logger: &activity_logger,
        };
        let result = game
            .kick(&ctx, &actor, &data.name, reason.as_deref())
            .await?;
        ApiResponse::new_serialized(result).ok()
    }
}
