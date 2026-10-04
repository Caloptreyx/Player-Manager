pub mod add {
    use crate::{
        context::{
            Context, JAVA_WHITELIST, Method, MutationResult, bedrock_known, known_xuid, uuid_is,
            xuid_is,
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
        /// UUID (Java) or XUID (Bedrock); looked up when missing.
        #[serde(default)]
        id: Option<String>,
        /// Bedrock only.
        #[serde(default)]
        ignores_player_limit: Option<bool>,
    }

    /// Adds a player to the whitelist (Java) or allowlist (Bedrock).
    #[utoipa::path(post, path = "/whitelist", responses(
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
        let (edition, method) = ctx.plan(&permissions)?;
        validate::name(edition, &data.name)?;
        let id = validate::optional_id(edition, data.id.as_deref())?;

        match (edition, method) {
            (Edition::Java, Method::Command) => {
                ctx.command(&activity_logger, &format!("whitelist add {}", data.name))
                    .await?;
            }
            (Edition::Java, Method::File) => {
                let (name, uuid) = ctx.resolve_java(&data.name, id).await?;
                let mut entries = ctx.load_list(JAVA_WHITELIST).await?;
                lists::upsert(
                    &mut entries,
                    |entry| uuid_is(entry, &uuid) || lists::has_name(entry, "name", &name),
                    |entry| {
                        entry.insert("uuid".into(), Value::from(uuid.as_str()));
                        entry.insert("name".into(), Value::from(name.as_str()));
                    },
                );
                ctx.write_list(JAVA_WHITELIST, &entries, user.uuid, &activity_logger)
                    .await?;
            }
            (Edition::Bedrock, _) => {
                let file = ctx.bedrock_allowlist();
                let mut entries = ctx.load_list(file).await?;
                let xuid = match id {
                    Some(id) => Some(id),
                    None => known_xuid(
                        &bedrock_known(&entries, &ctx.log_lines(&permissions).await),
                        &data.name,
                    ),
                };
                lists::upsert(
                    &mut entries,
                    |entry| {
                        lists::has_name(entry, "name", &data.name)
                            || xuid.as_deref().is_some_and(|xuid| xuid_is(entry, xuid))
                    },
                    |entry| {
                        let ignores = data
                            .ignores_player_limit
                            .or_else(|| entry.get("ignoresPlayerLimit").and_then(Value::as_bool))
                            .unwrap_or(false);
                        entry.insert("ignoresPlayerLimit".into(), Value::Bool(ignores));
                        entry.insert("name".into(), Value::from(data.name.as_str()));
                        if let Some(xuid) = &xuid {
                            entry.insert("xuid".into(), Value::from(xuid.as_str()));
                        }
                    },
                );
                ctx.write_list(file, &entries, user.uuid, &activity_logger)
                    .await?;
                ctx.reload(&activity_logger, ctx.bedrock_allowlist_reload())
                    .await?;
            }
        }

        MutationResult::respond(method, false)
    }
}

pub mod remove {
    use crate::{
        context::{Context, JAVA_WHITELIST, Method, MutationResult, player_not_found},
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

    /// Removes a player from the whitelist (Java) or allowlist (Bedrock).
    #[utoipa::path(delete, path = "/whitelist", responses(
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
        let (edition, method) = ctx.plan(&permissions)?;
        validate::name(edition, &data.name)?;

        if method == Method::Command {
            ctx.command(&activity_logger, &format!("whitelist remove {}", data.name))
                .await?;
            return MutationResult::respond(method, false);
        }

        let file = match edition {
            Edition::Java => JAVA_WHITELIST,
            Edition::Bedrock => ctx.bedrock_allowlist(),
        };
        let mut entries = ctx.load_list(file).await?;
        if lists::remove(&mut entries, |entry| {
            lists::has_name(entry, "name", &data.name)
        }) == 0
        {
            return Err(player_not_found());
        }
        ctx.write_list(file, &entries, user.uuid, &activity_logger)
            .await?;
        if edition == Edition::Bedrock {
            ctx.reload(&activity_logger, ctx.bedrock_allowlist_reload())
                .await?;
        }

        MutationResult::respond(method, false)
    }
}

pub mod enabled {
    use crate::{
        context::{Context, Method, MutationResult, PROPERTIES, ServerState},
        edition::Edition,
        properties,
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
        enabled: bool,
    }

    /// Turns the whitelist (Java) or allowlist (Bedrock) on or off.
    #[utoipa::path(put, path = "/whitelist/enabled", responses(
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
        let edition = ctx.require_edition()?;
        ctx.require_settled()?;
        let running = ctx.state == ServerState::Running;

        if edition == Edition::Java && running {
            permissions.has_server_permission("control.console")?;
            let command = if data.enabled {
                "whitelist on"
            } else {
                "whitelist off"
            };
            ctx.command(&activity_logger, command).await?;
            return MutationResult::respond(Method::Command, false);
        }

        permissions.has_server_permission("files.create")?;
        let content = ctx.read_text(PROPERTIES).await?.unwrap_or_default();
        let key = match edition {
            Edition::Java => "white-list",
            Edition::Bedrock
                if !properties::has(&content, "allow-list")
                    && properties::has(&content, "white-list") =>
            {
                "white-list"
            }
            Edition::Bedrock => "allow-list",
        };
        let updated = properties::set(&content, key, if data.enabled { "true" } else { "false" });
        if updated != content {
            ctx.write(PROPERTIES, updated, user.uuid, &activity_logger)
                .await?;
        }

        // a running Bedrock server only reads server.properties on start
        MutationResult::respond(Method::File, edition == Edition::Bedrock && running)
    }
}
