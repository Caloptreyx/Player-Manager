pub mod get {
    use crate::{
        context::Context,
        model::{GameInfo, Info, Overview},
    };
    use shared::{
        ApiError, GetState,
        models::{server::GetServer, user::GetPermissionManager},
        response::{ApiResponse, ApiResponseResult},
    };
    use std::collections::BTreeMap;

    /// The detected game, its lists and the server's player settings.
    #[utoipa::path(get, path = "/", responses(
        (status = OK, body = inline(Overview)),
        (status = UNAUTHORIZED, body = ApiError),
        (status = PAYLOAD_TOO_LARGE, body = ApiError),
    ), params(("server" = uuid::Uuid, description = "The server ID")))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        server: GetServer,
    ) -> ApiResponseResult {
        permissions.has_server_permission("files.read-content")?;

        let ctx = Context::load(&state, &server).await?;
        let Some(game) = ctx.game else {
            return ApiResponse::new_serialized(Overview {
                game: None,
                state: ctx.state,
                info: Info::default(),
                lists: BTreeMap::new(),
                known: Vec::new(),
                errors: Vec::new(),
            })
            .ok();
        };

        let (descriptor, contents) =
            tokio::try_join!(game.descriptor(&ctx), game.overview(&ctx, &permissions))?;
        ApiResponse::new_serialized(Overview {
            game: Some(GameInfo {
                id: game.id(),
                family: game.family(),
                descriptor,
            }),
            state: ctx.state,
            info: contents.info,
            lists: contents.lists,
            known: contents.known,
            errors: contents.errors,
        })
        .ok()
    }
}
