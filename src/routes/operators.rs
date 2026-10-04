pub mod add {
    use crate::{
        context::{
            BEDROCK_LEVELS, BEDROCK_PERMISSIONS, Context, JAVA_OPS, Level, Method, MutationResult,
            PROPERTIES, uuid_is, xuid_is,
        },
        edition::Edition,
        lists, properties, validate,
    };
    use serde::Deserialize;
    use serde_json::Value;
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
        /// UUID (Java) or XUID (Bedrock); looked up when missing.
        #[serde(default)]
        id: Option<String>,
        /// Java 1-4 (file edits only, default `op-permission-level`, else 4); Bedrock
        /// `operator` (default), `member` or `visitor`.
        #[serde(default)]
        level: Option<Level>,
        /// Java file edits only.
        #[serde(default)]
        bypasses_player_limit: Option<bool>,
    }

    /// Makes a player an operator (Java) or sets their permission level (Bedrock).
    #[utoipa::path(post, path = "/operators", responses(
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

        match edition {
            Edition::Java => {
                let level = match data.level {
                    None => None,
                    Some(Level::Number(level)) if (1..=4).contains(&level) => Some(level),
                    Some(_) => return Err(ApiResponse::error("the op level must be 1 to 4")),
                };
                if method == Method::Command {
                    ctx.command(&activity_logger, &format!("op {}", data.name))
                        .await?;
                    return MutationResult::respond(method, false);
                }

                let default_level = ctx
                    .read_text_lossy(PROPERTIES)
                    .await?
                    .and_then(|content| properties::get_u32(&content, "op-permission-level"))
                    .filter(|level| (1..=4).contains(level))
                    .unwrap_or(4);
                let (name, uuid) = ctx.resolve_java(&data.name, id).await?;
                let mut entries = ctx.load_list(JAVA_OPS).await?;
                lists::upsert(
                    &mut entries,
                    |entry| uuid_is(entry, &uuid) || lists::has_name(entry, "name", &name),
                    |entry| {
                        let level = level
                            .map(u64::from)
                            .or_else(|| {
                                entry
                                    .get("level")
                                    .and_then(Value::as_u64)
                                    .filter(|level| (1..=4).contains(level))
                            })
                            .unwrap_or(u64::from(default_level));
                        let bypasses = data
                            .bypasses_player_limit
                            .or_else(|| entry.get("bypassesPlayerLimit").and_then(Value::as_bool))
                            .unwrap_or(false);
                        entry.insert("uuid".into(), Value::from(uuid.as_str()));
                        entry.insert("name".into(), Value::from(name.as_str()));
                        entry.insert("level".into(), Value::from(level));
                        entry.insert("bypassesPlayerLimit".into(), Value::Bool(bypasses));
                    },
                );
                ctx.write_list(JAVA_OPS, &entries, user.uuid, &activity_logger)
                    .await?;
            }
            Edition::Bedrock => {
                let level = match &data.level {
                    None => "operator",
                    Some(Level::Name(level)) => BEDROCK_LEVELS
                        .into_iter()
                        .find(|known| *known == level.as_str())
                        .ok_or_else(|| {
                            ApiResponse::error(
                                "the permission level must be operator, member or visitor",
                            )
                        })?,
                    Some(Level::Number(_)) => {
                        return Err(ApiResponse::error(
                            "the permission level must be operator, member or visitor",
                        ));
                    }
                };
                let known = ctx.bedrock_known_players(&permissions).await?;
                let xuid = ctx.resolve_xuid(&data.name, id, &known).await?;
                let mut entries = ctx.load_list(BEDROCK_PERMISSIONS).await?;
                lists::upsert(
                    &mut entries,
                    |entry| xuid_is(entry, &xuid),
                    |entry| {
                        entry.insert("permission".into(), Value::from(level));
                        entry.insert("xuid".into(), Value::from(xuid.as_str()));
                    },
                );
                ctx.write_list(BEDROCK_PERMISSIONS, &entries, user.uuid, &activity_logger)
                    .await?;
                ctx.reload(&activity_logger, "permission reload").await?;
            }
        }

        MutationResult::respond(method, false)
    }
}

pub mod remove {
    use crate::{
        context::{
            BEDROCK_PERMISSIONS, Context, JAVA_OPS, Method, MutationResult, player_not_found,
            uuid_is, xuid_is,
        },
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
        response::{ApiResponse, ApiResponseResult},
    };
    use utoipa::ToSchema;

    #[derive(ToSchema, Deserialize)]
    pub struct Payload {
        #[serde(default)]
        name: Option<String>,
        #[serde(default)]
        id: Option<String>,
    }

    /// Removes an operator (Java) or a permission entry (Bedrock), by name or id.
    #[utoipa::path(delete, path = "/operators", responses(
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
        if let Some(name) = &data.name {
            validate::name(edition, name)?;
        }
        let id = validate::optional_id(edition, data.id.as_deref())?;
        if data.name.is_none() && id.is_none() {
            return Err(ApiResponse::error("a name or an id is required"));
        }

        match edition {
            Edition::Java => {
                if method == Method::Command {
                    let name = match (&data.name, &id) {
                        (Some(name), _) => name.clone(),
                        (None, Some(uuid)) => {
                            let entries = ctx.load_list(JAVA_OPS).await?;
                            let name = lists::objects(&entries)
                                .find(|entry| uuid_is(entry, uuid))
                                .and_then(|entry| lists::string(entry, "name"))
                                .filter(|name| crate::validate::java_name(name))
                                .map(str::to_string);
                            name.ok_or_else(player_not_found)?
                        }
                        (None, None) => unreachable!("checked above"),
                    };
                    ctx.command(&activity_logger, &format!("deop {name}"))
                        .await?;
                    return MutationResult::respond(method, false);
                }

                let mut entries = ctx.load_list(JAVA_OPS).await?;
                let removed = lists::remove(&mut entries, |entry| {
                    id.as_deref().is_some_and(|uuid| uuid_is(entry, uuid))
                        || data
                            .name
                            .as_deref()
                            .is_some_and(|name| lists::has_name(entry, "name", name))
                });
                if removed == 0 {
                    return Err(player_not_found());
                }
                ctx.write_list(JAVA_OPS, &entries, user.uuid, &activity_logger)
                    .await?;
            }
            Edition::Bedrock => {
                let xuid = match (id, &data.name) {
                    (Some(id), _) => id,
                    (None, Some(name)) => {
                        let known = ctx.bedrock_known_players(&permissions).await?;
                        ctx.resolve_xuid(name, None, &known).await?
                    }
                    (None, None) => unreachable!("checked above"),
                };
                let mut entries = ctx.load_list(BEDROCK_PERMISSIONS).await?;
                if lists::remove(&mut entries, |entry| xuid_is(entry, &xuid)) == 0 {
                    return Err(player_not_found());
                }
                ctx.write_list(BEDROCK_PERMISSIONS, &entries, user.uuid, &activity_logger)
                    .await?;
                ctx.reload(&activity_logger, "permission reload").await?;
            }
        }

        MutationResult::respond(method, false)
    }
}
