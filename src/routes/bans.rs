pub mod add {
    use crate::{
        context::{
            BAN_REASON, Context, JAVA_BANS, Method, MutationResult, ban_created, java_only, uuid_is,
        },
        edition::Edition,
        lists, validate,
    };
    use serde::Deserialize;
    use serde_json::Value;
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::{GetPermissionManager, GetUser},
        },
        response::ApiResponseResult,
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        name: String,
        /// UUID; looked up when missing.
        #[serde(default)]
        id: Option<String>,
        #[serde(default)]
        reason: Option<String>,
    }

    /// Bans a player (Java only).
    #[utoipa::path(post, path = "/bans", responses(
        (status = OK, body = inline(MutationResult)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
        (status = CONFLICT, body = ApiError),
        (status = UNPROCESSABLE_ENTITY, body = ApiError),
        (status = BAD_GATEWAY, body = ApiError),
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
        if ctx.require_edition()? == Edition::Bedrock {
            return Err(java_only());
        }
        let (edition, method) = ctx.plan(&permissions)?;
        validate::name(edition, &data.name)?;
        let id = validate::optional_id(edition, data.id.as_deref())?;
        let reason = validate::reason(data.reason.as_deref())?;

        if method == Method::Command {
            let command = match &reason {
                Some(reason) => format!("ban {} {reason}", data.name),
                None => format!("ban {}", data.name),
            };
            ctx.command(&activity_logger, &command).await?;
            return MutationResult::respond(method, false);
        }

        let (name, uuid) = ctx.resolve_java(&data.name, id).await?;
        let created = ban_created();
        let reason = reason.as_deref().unwrap_or(BAN_REASON);
        let mut entries = ctx.load_list(JAVA_BANS).await?;
        lists::upsert(
            &mut entries,
            |entry| uuid_is(entry, &uuid) || lists::has_name(entry, "name", &name),
            |entry| {
                entry.insert("uuid".into(), Value::from(uuid.as_str()));
                entry.insert("name".into(), Value::from(name.as_str()));
                entry.insert("created".into(), Value::from(created.as_str()));
                entry.insert("source".into(), Value::from("Server"));
                entry.insert("expires".into(), Value::from("forever"));
                entry.insert("reason".into(), Value::from(reason));
            },
        );
        ctx.write_list(JAVA_BANS, &entries, user.uuid, &activity_logger)
            .await?;

        MutationResult::respond(method, false)
    }
}

pub mod remove {
    use crate::{
        context::{Context, JAVA_BANS, Method, MutationResult, java_only, player_not_found},
        edition::Edition,
        lists, validate,
    };
    use serde::Deserialize;
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::{GetPermissionManager, GetUser},
        },
        response::ApiResponseResult,
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        name: String,
    }

    /// Pardons a banned player (Java only).
    #[utoipa::path(delete, path = "/bans", responses(
        (status = OK, body = inline(MutationResult)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
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
        if ctx.require_edition()? == Edition::Bedrock {
            return Err(java_only());
        }
        let (edition, method) = ctx.plan(&permissions)?;
        validate::name(edition, &data.name)?;

        if method == Method::Command {
            ctx.command(&activity_logger, &format!("pardon {}", data.name))
                .await?;
            return MutationResult::respond(method, false);
        }

        let mut entries = ctx.load_list(JAVA_BANS).await?;
        if lists::remove(&mut entries, |entry| {
            lists::has_name(entry, "name", &data.name)
        }) == 0
        {
            return Err(player_not_found());
        }
        ctx.write_list(JAVA_BANS, &entries, user.uuid, &activity_logger)
            .await?;

        MutationResult::respond(method, false)
    }
}
