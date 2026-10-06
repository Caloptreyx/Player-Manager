//! Profile actions: validated against the closed sets of the API, then run as commands when
//! the player is online and as edits of their saved data when they are not.
use crate::{
    games::{ProfileAction, ProfileMode},
    model::{Container, Gamemode, Online, Player, ServerState, Slot},
    validate,
};
use regex::Regex;
use serde::Deserialize;
use shared::response::ApiResponse;
use std::sync::LazyLock;
use utoipa::ToSchema;

const MAX_XP_LEVEL: u32 = 21863;
const MAX_GIVE_COUNT: u32 = 6400;
const TRANSITION: &str = "the server is starting or stopping, try again in a moment";
const CANNOT_TELL: &str = "could not tell whether the player is online";
const MUST_BE_ONLINE: &str = "the player must be online";

/// An item id; without a namespace it is a `minecraft:` one.
static ITEM: LazyLock<Regex> =
    LazyLock::new(|| validate::compile(r"^(?:[a-z0-9_.-]+:)?[a-z0-9_./-]+$"));

#[derive(ToSchema, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Payload {
    /// Empties one slot.
    ClearSlot {
        slot: String,
    },
    /// Empties the inventory (with hotbar, armor and offhand) or the ender chest.
    ClearContainer {
        container: Container,
    },
    SetGamemode {
        gamemode: Gamemode,
    },
    /// 0 to 21863; the progress to the next level is reset.
    SetXpLevel {
        level: i64,
    },
    /// Online players only; 1 to 6400 items.
    Give {
        item: String,
        count: i64,
    },
}

/// `value` when it is `min` to `max`.
fn bounded(value: i64, min: u32, max: u32, what: &str) -> Result<u32, ApiResponse> {
    u32::try_from(value)
        .ok()
        .filter(|value| (min..=max).contains(value))
        .ok_or_else(|| ApiResponse::error(format!("the {what} must be {min} to {max}")))
}

/// The action `payload` asks for, when its slot, level, item and count are valid.
fn action(payload: Payload) -> Result<ProfileAction, ApiResponse> {
    Ok(match payload {
        Payload::ClearSlot { slot } => ProfileAction::ClearSlot(
            Slot::parse(&slot).ok_or_else(|| ApiResponse::error("unknown slot"))?,
        ),
        Payload::ClearContainer { container } => ProfileAction::ClearContainer(container),
        Payload::SetGamemode { gamemode } => ProfileAction::SetGamemode(gamemode),
        Payload::SetXpLevel { level } => {
            ProfileAction::SetXpLevel(bounded(level, 0, MAX_XP_LEVEL, "level")?)
        }
        Payload::Give { item, count } => {
            if !ITEM.is_match(&item) {
                return Err(ApiResponse::error("invalid item id"));
            }
            let item = if item.contains(':') {
                item
            } else {
                format!("minecraft:{item}")
            };
            ProfileAction::Give {
                item,
                count: bounded(count, 1, MAX_GIVE_COUNT, "count")?,
            }
        }
    })
}

/// How an action reaches the player `id` (`name` when known): file edits while the server is
/// offline; while it runs, commands when the player is among the players online (by id, or by
/// name for players listed without one) and file edits when they certainly are not.
fn mode(
    state: ServerState,
    online: Option<&Online>,
    id: &str,
    name: Option<&str>,
) -> Result<ProfileMode, &'static str> {
    match state {
        ServerState::Offline => return Ok(ProfileMode::File),
        ServerState::Starting | ServerState::Stopping => return Err(TRANSITION),
        ServerState::Running => {}
    }
    let online = online.filter(|online| online.complete).ok_or(CANNOT_TELL)?;
    let is_player = |player: &Player| match &player.id {
        Some(player_id) => player_id == id,
        None => name.is_some_and(|name| player.name.eq_ignore_ascii_case(name)),
    };
    if let Some(player) = online.players.iter().find(|&player| is_player(player)) {
        return Ok(ProfileMode::Live {
            name: player.name.clone(),
        });
    }
    // without a name, players listed without an id could be them
    if name.is_some() || online.players.iter().all(|player| player.id.is_some()) {
        Ok(ProfileMode::File)
    } else {
        Err(CANNOT_TELL)
    }
}

pub mod post {
    use super::{MUST_BE_ONLINE, Payload, action, mode};
    use crate::{
        context::{Actor, Context, authorize, conflict, unsupported},
        games::{ProfileAction, ProfileMode},
        model::{MutationResult, ServerState},
        routes::lists,
        validate,
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

    /// Acts on a player: by commands while they are online, by editing their saved data
    /// otherwise (`give` needs them online).
    #[utoipa::path(post, path = "/profiles/{id}/actions", responses(
        (status = OK, body = inline(MutationResult)),
        (status = BAD_REQUEST, body = ApiError),
        (status = UNAUTHORIZED, body = ApiError),
        (status = NOT_FOUND, body = ApiError),
        (status = CONFLICT, body = ApiError),
        (status = PAYLOAD_TOO_LARGE, body = ApiError),
        (status = UNPROCESSABLE_ENTITY, body = ApiError),
        (status = BAD_GATEWAY, body = ApiError),
        (status = GATEWAY_TIMEOUT, body = ApiError),
    ), params(
        ("server" = uuid::Uuid, description = "The server ID"),
        ("id" = String, Path, description = "The player id"),
    ), request_body = inline(Payload))]
    pub async fn route(
        state: GetState,
        permissions: GetPermissionManager,
        user: GetUser,
        server: GetServer,
        activity_logger: GetServerActivityLogger,
        Path((_server, player)): Path<(String, String)>,
        shared::Payload(data): shared::Payload<Payload>,
    ) -> ApiResponseResult {
        let ctx = Context::load(&state, &server).await?;
        let game = ctx.require_game()?;
        let descriptor = game.descriptor(&ctx).await?;
        let profiles = descriptor
            .profiles
            .ok_or_else(|| unsupported("player profiles"))?;
        authorize(&profiles.view, &permissions)?;
        let id = lists::id(game, &descriptor, &player)?;
        let action = action(data)?;
        let profile = game.profile(&ctx, &id).await?;

        let actor = Actor {
            permissions: &permissions,
            user: user.uuid,
            activity_logger: &activity_logger,
        };
        let online = if ctx.state == ServerState::Running {
            game.online(&ctx, &actor).await.ok()
        } else {
            None
        };
        let mode =
            mode(ctx.state, online.as_ref(), &id, profile.name.as_deref()).map_err(conflict)?;
        let capability = match &mode {
            ProfileMode::Live { name } => {
                validate::player_name(descriptor.player_name.pattern, name)
                    .map_err(|_| conflict("the player's name cannot be used in commands"))?;
                profiles.edit_live
            }
            ProfileMode::File if matches!(action, ProfileAction::Give { .. }) => {
                return Err(conflict(MUST_BE_ONLINE));
            }
            ProfileMode::File => profiles.edit_offline,
        }
        .ok_or_else(|| unsupported("editing player profiles"))?;
        authorize(&capability, &permissions)?;

        let result = game.profile_action(&ctx, &actor, &id, mode, action).await?;
        ApiResponse::new_serialized(result).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Armor, OnlineSource};

    const ID: &str = "069a79f4-44e9-4726-a5be-fca90e38aaf5";
    const OTHER: &str = "853c80ef-3c37-49fd-aa49-938b674adae6";

    fn player(name: &str, id: Option<&str>) -> Player {
        Player {
            name: name.into(),
            id: id.map(str::to_string),
        }
    }

    fn online(players: Vec<Player>, complete: bool) -> Online {
        Online {
            count: 3,
            max: 20,
            players,
            source: OnlineSource::Query,
            complete,
        }
    }

    fn live(name: &str) -> Result<ProfileMode, &'static str> {
        Ok(ProfileMode::Live { name: name.into() })
    }

    #[test]
    fn validates_actions() {
        let slot = |slot: &str| action(Payload::ClearSlot { slot: slot.into() }).ok();
        assert_eq!(
            slot("hotbar.8"),
            Some(ProfileAction::ClearSlot(Slot::Hotbar(8)))
        );
        assert_eq!(
            slot("armor.legs"),
            Some(ProfileAction::ClearSlot(Slot::Armor(Armor::Legs)))
        );
        assert_eq!(
            slot("weapon.offhand"),
            Some(ProfileAction::ClearSlot(Slot::Offhand))
        );
        assert_eq!(
            slot("enderchest.26"),
            Some(ProfileAction::ClearSlot(Slot::EnderChest(26)))
        );
        for bad in [
            "hotbar.9",
            "inventory.27",
            "enderchest.-1",
            "hotbar.+1",
            "hotbar.01",
            "hotbar.",
            "armor.body",
            "weapon.mainhand",
            "container.0",
            "hotbar.0 with minecraft:tnt",
        ] {
            assert_eq!(slot(bad), None, "{bad}");
        }

        let level = |level| action(Payload::SetXpLevel { level }).ok();
        assert_eq!(level(0), Some(ProfileAction::SetXpLevel(0)));
        assert_eq!(level(21863), Some(ProfileAction::SetXpLevel(21863)));
        assert_eq!(level(21864), None);
        assert_eq!(level(-1), None);

        let give = |item: &str, count| {
            action(Payload::Give {
                item: item.into(),
                count,
            })
            .ok()
        };
        assert_eq!(
            give("diamond", 64),
            Some(ProfileAction::Give {
                item: "minecraft:diamond".into(),
                count: 64,
            })
        );
        assert_eq!(
            give("mymod:tools/hammer", 6400),
            Some(ProfileAction::Give {
                item: "mymod:tools/hammer".into(),
                count: 6400,
            })
        );
        assert_eq!(give("diamond", 0), None);
        assert_eq!(give("diamond", 6401), None);
        for bad in [
            "Diamond",
            "minecraft:diamond{}",
            "diamond 64",
            "a:b:c",
            "minecraft:diamond\nop Steve",
            "",
        ] {
            assert_eq!(give(bad, 1), None, "{bad:?}");
        }
    }

    #[test]
    fn decides_between_commands_and_file_edits() {
        let everyone = online(
            vec![player("Notch", Some(ID)), player("jeb_", Some(OTHER))],
            true,
        );
        let unnamed = online(vec![player("Notch", None), player("jeb_", None)], true);

        assert_eq!(
            mode(ServerState::Offline, None, ID, None),
            Ok(ProfileMode::File)
        );
        for state in [ServerState::Starting, ServerState::Stopping] {
            assert_eq!(mode(state, Some(&everyone), ID, None), Err(TRANSITION));
        }
        // no answer, or one that may lack the player
        assert_eq!(
            mode(ServerState::Running, None, ID, Some("Notch")),
            Err(CANNOT_TELL)
        );
        assert_eq!(
            mode(
                ServerState::Running,
                Some(&online(vec![player("Notch", Some(ID))], false)),
                ID,
                Some("Notch")
            ),
            Err(CANNOT_TELL)
        );
        // online by id, whatever the name
        assert_eq!(
            mode(ServerState::Running, Some(&everyone), ID, Some("OldName")),
            live("Notch")
        );
        // online by name when the list has no ids
        assert_eq!(
            mode(ServerState::Running, Some(&unnamed), ID, Some("NOTCH")),
            live("Notch")
        );
        // a player with the same name but another id is someone else
        assert_eq!(
            mode(ServerState::Running, Some(&everyone), OTHER, Some("Notch")),
            live("jeb_")
        );
        assert_eq!(
            mode(
                ServerState::Running,
                Some(&online(vec![player("Notch", Some(OTHER))], true)),
                ID,
                Some("Notch")
            ),
            Ok(ProfileMode::File)
        );
        // offline: not listed by id, and the name (when known) is not listed either
        assert_eq!(
            mode(ServerState::Running, Some(&unnamed), ID, Some("Alex")),
            Ok(ProfileMode::File)
        );
        assert_eq!(
            mode(
                ServerState::Running,
                Some(&online(vec![player("jeb_", Some(OTHER))], true)),
                ID,
                None
            ),
            Ok(ProfileMode::File)
        );
        assert_eq!(
            mode(ServerState::Running, Some(&online(vec![], true)), ID, None),
            Ok(ProfileMode::File)
        );
        // without a name, a player listed without an id could be them
        assert_eq!(
            mode(ServerState::Running, Some(&unnamed), ID, None),
            Err(CANNOT_TELL)
        );
    }
}
