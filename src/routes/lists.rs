//! List adds and removes, validated against the game's [`ListSpec`] before the game runs them.
use crate::{
    context::list_not_supported,
    games::{Add, Game, Selector, Subject},
    model::{Descriptor, IdField, ListKind, ListSpec, ListTarget},
    validate,
};
use serde::Deserialize;
use shared::response::ApiResponse;
use utoipa::ToSchema;

#[derive(ToSchema, Deserialize, Default)]
pub struct AddPayload {
    #[serde(default)]
    name: Option<String>,
    /// The game's player id; looked up when missing.
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    ip: Option<String>,
    /// One of the list's level options.
    #[serde(default)]
    level: Option<String>,
    #[serde(default)]
    bypasses_player_limit: Option<bool>,
    #[serde(default)]
    reason: Option<String>,
}

#[derive(ToSchema, Deserialize, Default)]
pub struct RemovePayload {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    ip: Option<String>,
}

/// The game's list named `kind` (404 when it has none).
fn spec<'a>(descriptor: &'a Descriptor, kind: &str) -> Result<&'a ListSpec, ApiResponse> {
    let kind = ListKind::parse(kind).ok_or_else(list_not_supported)?;
    descriptor
        .lists
        .iter()
        .find(|spec| spec.kind == kind)
        .ok_or_else(list_not_supported)
}

fn not_accepted(field: &str) -> ApiResponse {
    ApiResponse::error(format!("`{field}` is not accepted for this list"))
}

fn name(descriptor: &Descriptor, name: String) -> Result<String, ApiResponse> {
    validate::player_name(descriptor.player_name.pattern, &name)?;
    Ok(name)
}

fn id(game: &dyn Game, descriptor: &Descriptor, id: &str) -> Result<String, ApiResponse> {
    validate::player_id(descriptor.player_id.pattern, id)?;
    Ok(game.normalize_id(id))
}

/// The add `payload` asks for, when the list accepts it in the current mode.
fn add_request(
    game: &dyn Game,
    descriptor: &Descriptor,
    spec: &ListSpec,
    payload: AddPayload,
) -> Result<Add, ApiResponse> {
    let subject = match spec.target {
        ListTarget::Player => {
            if payload.ip.is_some() {
                return Err(not_accepted("ip"));
            }
            let player = payload
                .name
                .ok_or_else(|| ApiResponse::error("a name is required"))?;
            let player_id = match payload.id {
                None => None,
                Some(_) if spec.id == IdField::None => return Err(not_accepted("id")),
                Some(value) => Some(id(game, descriptor, &value)?),
            };
            Subject::Player {
                name: name(descriptor, player)?,
                id: player_id,
            }
        }
        ListTarget::Ip => {
            if payload.name.is_some() {
                return Err(not_accepted("name"));
            }
            if payload.id.is_some() {
                return Err(not_accepted("id"));
            }
            let ip = payload
                .ip
                .ok_or_else(|| ApiResponse::error("an IP address is required"))?;
            Subject::Ip(validate::ip(&ip)?)
        }
    };
    let level = match (payload.level, spec.levels) {
        (None, _) => None,
        (Some(_), None) => return Err(not_accepted("level")),
        (Some(level), Some(levels)) if levels.options.contains(&level.as_str()) => Some(level),
        (Some(_), Some(levels)) => {
            return Err(ApiResponse::error(format!(
                "the level must be one of {}",
                levels.options.join(", ")
            )));
        }
    };
    if payload.bypasses_player_limit.is_some() && !spec.bypasses_player_limit {
        return Err(not_accepted("bypasses_player_limit"));
    }
    let reason = validate::reason(payload.reason.as_deref())?;
    if reason.is_some() && !spec.reason {
        return Err(not_accepted("reason"));
    }
    Ok(Add {
        subject,
        level,
        bypasses_player_limit: payload.bypasses_player_limit,
        reason,
    })
}

/// The entry `payload` selects: by name and/or id for player lists, by address for IP lists.
fn remove_request(
    game: &dyn Game,
    descriptor: &Descriptor,
    spec: &ListSpec,
    payload: RemovePayload,
) -> Result<Selector, ApiResponse> {
    match spec.target {
        ListTarget::Player => {
            if payload.ip.is_some() {
                return Err(not_accepted("ip"));
            }
            if payload.name.is_none() && payload.id.is_none() {
                return Err(ApiResponse::error("a name or an id is required"));
            }
            Ok(Selector::Player {
                name: payload
                    .name
                    .map(|player| name(descriptor, player))
                    .transpose()?,
                id: payload
                    .id
                    .map(|value| id(game, descriptor, &value))
                    .transpose()?,
            })
        }
        ListTarget::Ip => {
            if payload.name.is_some() {
                return Err(not_accepted("name"));
            }
            if payload.id.is_some() {
                return Err(not_accepted("id"));
            }
            let ip = payload
                .ip
                .ok_or_else(|| ApiResponse::error("an IP address is required"))?;
            Ok(Selector::Ip(validate::ip(&ip)?))
        }
    }
}

pub mod post {
    use super::{AddPayload, add_request, spec};
    use crate::{
        context::{Actor, Context, authorize, unsupported},
        model::{ListKind, MutationResult},
    };
    use axum::extract::Path;
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::{GetPermissionManager, GetUser},
        },
        response::{ApiResponse, ApiResponseResult},
    };

    /// Adds a player (or an IP address) to one of the game's lists.
    #[utoipa::path(post, path = "/lists/{kind}", responses(
        (status = OK, body = inline(MutationResult)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
        (status = CONFLICT, body = ApiError),
        (status = UNPROCESSABLE_ENTITY, body = ApiError),
        (status = BAD_GATEWAY, body = ApiError),
    ), params(
        ("server" = uuid::Uuid, description = "The server ID"),
        ("kind" = inline(ListKind), Path, description = "The list"),
    ), request_body = inline(AddPayload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        user: GetUser,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
        Path((_server, kind)): Path<(String, String)>,
        shared::Payload(data): shared::Payload<AddPayload>,
    ) -> ApiResponseResult {
        let ctx = Context::load(&state, &server).await?;
        let game = ctx.require_game()?;
        let descriptor = game.descriptor(&ctx).await?;
        let edit = descriptor
            .edit
            .ok_or_else(|| unsupported("editing lists"))?;
        let spec = spec(&descriptor, &kind)?;
        authorize(&edit.access, &permissions)?;
        let add = add_request(game, &descriptor, spec, data)?;

        let actor = Actor {
            permissions: &permissions,
            user: user.uuid,
            activity_logger: &activity_logger,
        };
        let result = game.add(&ctx, &actor, edit.method, spec, add).await?;
        ApiResponse::new_serialized(result).ok()
    }
}

pub mod delete {
    use super::{RemovePayload, remove_request, spec};
    use crate::{
        context::{Actor, Context, authorize, unsupported},
        model::{ListKind, MutationResult},
    };
    use axum::extract::Path;
    use shared::{
        ApiError, GetState,
        models::{
            server::{GetServer, GetServerActivityLogger},
            user::{GetPermissionManager, GetUser},
        },
        response::{ApiResponse, ApiResponseResult},
    };

    /// Removes a player (by name and/or id) or an IP address from one of the game's lists.
    #[utoipa::path(delete, path = "/lists/{kind}", responses(
        (status = OK, body = inline(MutationResult)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
        (status = CONFLICT, body = ApiError),
        (status = UNPROCESSABLE_ENTITY, body = ApiError),
        (status = BAD_GATEWAY, body = ApiError),
    ), params(
        ("server" = uuid::Uuid, description = "The server ID"),
        ("kind" = inline(ListKind), Path, description = "The list"),
    ), request_body = inline(RemovePayload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        user: GetUser,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
        Path((_server, kind)): Path<(String, String)>,
        shared::Payload(data): shared::Payload<RemovePayload>,
    ) -> ApiResponseResult {
        let ctx = Context::load(&state, &server).await?;
        let game = ctx.require_game()?;
        let descriptor = game.descriptor(&ctx).await?;
        let edit = descriptor
            .edit
            .ok_or_else(|| unsupported("editing lists"))?;
        let spec = spec(&descriptor, &kind)?;
        authorize(&edit.access, &permissions)?;
        let selector = remove_request(game, &descriptor, spec, data)?;

        let actor = Actor {
            permissions: &permissions,
            user: user.uuid,
            activity_logger: &activity_logger,
        };
        let result = game
            .remove(&ctx, &actor, edit.method, spec, selector)
            .await?;
        ApiResponse::new_serialized(result).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        games::minecraft::{
            bedrock::{self, MinecraftBedrock},
            java::{self, MinecraftJava},
        },
        model::ServerState,
    };

    fn java_add(state: ServerState, kind: &str, payload: AddPayload) -> Result<Add, ApiResponse> {
        let descriptor = java::descriptor(state, 4);
        let spec = *spec(&descriptor, kind).unwrap();
        add_request(&MinecraftJava, &descriptor, &spec, payload)
    }

    fn named(name: &str) -> AddPayload {
        AddPayload {
            name: Some(name.into()),
            ..AddPayload::default()
        }
    }

    #[test]
    fn unknown_and_missing_lists_are_not_found() {
        let descriptor = java::descriptor(ServerState::Running, 4);
        assert!(spec(&descriptor, "ip_bans").is_ok());
        assert!(spec(&descriptor, "ip-bans").is_err());
        let bedrock = bedrock::descriptor(ServerState::Running);
        assert!(spec(&bedrock, "bans").is_err());
        assert!(spec(&bedrock, "operators").is_ok());
    }

    #[test]
    fn commands_take_no_ids_levels_or_flags() {
        let running = ServerState::Running;
        assert!(java_add(running, "operators", named("Notch")).is_ok());
        for payload in [
            AddPayload {
                id: Some("069a79f4-44e9-4726-a5be-fca90e38aaf5".into()),
                ..named("Notch")
            },
            AddPayload {
                level: Some("4".into()),
                ..named("Notch")
            },
            AddPayload {
                bypasses_player_limit: Some(true),
                ..named("Notch")
            },
            AddPayload {
                reason: Some("griefing".into()),
                ..named("Notch")
            },
        ] {
            assert!(java_add(running, "operators", payload).is_err());
        }
        // a blank reason is no reason
        assert!(
            java_add(
                running,
                "whitelist",
                AddPayload {
                    reason: Some(" ".into()),
                    ..named("Notch")
                }
            )
            .is_ok()
        );
    }

    #[test]
    fn file_edits_take_normalized_ids_and_known_levels() {
        let offline = ServerState::Offline;
        let add = java_add(
            offline,
            "operators",
            AddPayload {
                id: Some("069A79F444E94726A5BEFCA90E38AAF5".into()),
                level: Some("2".into()),
                bypasses_player_limit: Some(true),
                ..named(".Steve")
            },
        )
        .unwrap();
        let Subject::Player { name, id } = add.subject else {
            panic!("expected a player");
        };
        assert_eq!(name, ".Steve");
        assert_eq!(id.as_deref(), Some("069a79f4-44e9-4726-a5be-fca90e38aaf5"));
        assert_eq!(add.level.as_deref(), Some("2"));
        assert_eq!(add.bypasses_player_limit, Some(true));

        for payload in [
            AddPayload {
                level: Some("5".into()),
                ..named("Notch")
            },
            AddPayload {
                id: Some("not-a-uuid".into()),
                ..named("Notch")
            },
            named("Not a name"),
            AddPayload::default(),
        ] {
            assert!(java_add(offline, "operators", payload).is_err());
        }
    }

    #[test]
    fn ip_lists_take_addresses_only() {
        let ip = |ip: &str| AddPayload {
            ip: Some(ip.into()),
            reason: Some("spam".into()),
            ..AddPayload::default()
        };
        let add = java_add(ServerState::Running, "ip_bans", ip(" 10.0.0.1 ")).unwrap();
        assert!(matches!(add.subject, Subject::Ip(ip) if ip.to_string() == "10.0.0.1"));
        assert_eq!(add.reason.as_deref(), Some("spam"));
        assert!(java_add(ServerState::Running, "ip_bans", ip("10.0.0")).is_err());
        assert!(
            java_add(
                ServerState::Running,
                "ip_bans",
                AddPayload {
                    name: Some("Notch".into()),
                    ..ip("10.0.0.1")
                }
            )
            .is_err()
        );
        assert!(java_add(ServerState::Running, "bans", ip("10.0.0.1")).is_err());
    }

    #[test]
    fn removals_need_a_name_or_an_id() {
        let descriptor = bedrock::descriptor(ServerState::Offline);
        let operators = *spec(&descriptor, "operators").unwrap();
        let remove = |payload| remove_request(&MinecraftBedrock, &descriptor, &operators, payload);
        assert!(remove(RemovePayload::default()).is_err());
        assert!(matches!(
            remove(RemovePayload {
                id: Some("2535428692371234".into()),
                ..RemovePayload::default()
            }),
            Ok(Selector::Player {
                name: None,
                id: Some(_)
            })
        ));
        assert!(
            remove(RemovePayload {
                name: Some("Some Guy".into()),
                ..RemovePayload::default()
            })
            .is_ok()
        );
        assert!(
            remove(RemovePayload {
                id: Some("12a".into()),
                ..RemovePayload::default()
            })
            .is_err()
        );
        assert!(
            remove(RemovePayload {
                ip: Some("10.0.0.1".into()),
                name: Some("Some Guy".into()),
                ..RemovePayload::default()
            })
            .is_err()
        );
    }
}
