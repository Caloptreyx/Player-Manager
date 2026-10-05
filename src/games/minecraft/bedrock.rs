//! Minecraft: Bedrock Edition (Bedrock Dedicated Server). Lists are always edited as files; a
//! running server is told to reread them. BDS has no ban list.
use super::{
    BEDROCK_NAME, FAMILY, FILES, PROPERTIES, SCORE_BEDROCK_BINARY, SCORE_BEDROCK_HINTS, XUID,
    console, lookup, properties,
};
use crate::{
    context::{Actor, Context, list_not_supported, player_not_found, unprocessable},
    games::{Add, Contents, Game, Selector, Subject},
    lists::{self, Object},
    model::{
        Capability, Descriptor, Entry, IdField, Info, Levels, ListKind, ListSpec, ListTarget,
        Method, MethodCapability, MutationResult, Online, Player, PlayerId, PlayerName,
        ServerState,
    },
};
use serde_json::Value;
use shared::{models::user::PermissionManager, response::ApiResponse};
use std::collections::{BTreeMap, HashMap};

pub const ID: &str = "minecraft_bedrock";

const ALLOWLIST: &str = "allowlist.json";
const LEGACY_ALLOWLIST: &str = "whitelist.json";
const PERMISSIONS: &str = "permissions.json";
const BINARIES: [&str; 2] = ["bedrock_server", "bedrock_server.exe"];

/// Permission levels, `permission` in `permissions.json`.
const LEVELS: &[&str] = &["operator", "member", "visitor"];

const FILES_AND_CONSOLE: &[&str] = &["files.create", "control.console"];

pub struct MinecraftBedrock;

fn detect(files: &[&str], hints: &[&str]) -> Option<u8> {
    let has = |name: &str| files.contains(&name);
    if BINARIES.iter().any(|name| has(name)) {
        Some(SCORE_BEDROCK_BINARY)
    } else if hints
        .iter()
        .any(|hint| hint.to_ascii_lowercase().contains("bedrock"))
        || has(ALLOWLIST)
        || has(PERMISSIONS)
    {
        Some(SCORE_BEDROCK_HINTS)
    } else {
        None
    }
}

/// File edits in any settled state, plus a reload command (and so `control.console`) while
/// running.
pub(crate) fn descriptor(state: ServerState) -> Descriptor {
    let blocked = super::transition(state);
    let player = |kind| ListSpec {
        kind,
        target: ListTarget::Player,
        id: IdField::Optional,
        reason: false,
        levels: None,
        bypasses_player_limit: false,
    };

    Descriptor {
        player_name: PlayerName {
            pattern: &BEDROCK_NAME,
        },
        player_id: PlayerId {
            kind: "xuid",
            pattern: &XUID,
        },
        lists: vec![
            ListSpec {
                bypasses_player_limit: true,
                ..player(ListKind::Whitelist)
            },
            ListSpec {
                levels: Some(Levels {
                    options: LEVELS,
                    default: LEVELS[0],
                }),
                ..player(ListKind::Operators)
            },
        ],
        edit: Some(MethodCapability {
            access: Capability {
                requires: if state == ServerState::Running {
                    FILES_AND_CONSOLE
                } else {
                    FILES
                },
                visible_with: FILES,
                blocked,
            },
            method: Method::File,
        }),
        whitelist_toggle: Some(MethodCapability {
            access: Capability {
                requires: FILES,
                visible_with: FILES,
                blocked,
            },
            method: Method::File,
        }),
        online: Some(super::online_capability(state)),
        kick: Some(super::kick_capability(state)),
    }
}

/// The allowlist file: `allowlist.json`, or the legacy `whitelist.json` when only it exists.
fn allowlist(ctx: &Context<'_>) -> &'static str {
    if !ctx.has(ALLOWLIST) && ctx.has(LEGACY_ALLOWLIST) {
        LEGACY_ALLOWLIST
    } else {
        ALLOWLIST
    }
}

/// The command that makes a running server reread its allowlist file.
fn allowlist_reload(ctx: &Context<'_>) -> &'static str {
    if allowlist(ctx) == LEGACY_ALLOWLIST {
        "whitelist reload"
    } else {
        "allowlist reload"
    }
}

fn xuid_is(entry: &Object, xuid: &str) -> bool {
    lists::text(entry, "xuid").is_some_and(|found| found == xuid)
}

/// Players with a known XUID: allowlist entries, then console join lines (newer joins win);
/// one entry per name, ignoring case.
fn known(allowlist: &[Value], log_lines: &[String]) -> Vec<Player> {
    let from_allowlist = lists::objects(allowlist).filter_map(|entry| {
        let xuid = lists::text(entry, "xuid").filter(|xuid| XUID.is_match(xuid))?;
        Some((lists::string(entry, "name")?.to_string(), xuid))
    });
    let mut players: Vec<Player> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for (name, xuid) in from_allowlist.chain(console::connected_players(log_lines)) {
        match index.get(&name.to_ascii_lowercase()) {
            Some(&position) => {
                players[position] = Player {
                    name,
                    id: Some(xuid),
                }
            }
            None => {
                index.insert(name.to_ascii_lowercase(), players.len());
                players.push(Player {
                    name,
                    id: Some(xuid),
                });
            }
        }
    }
    players
}

/// The XUID of `name` among `known` players.
fn known_xuid(known: &[Player], name: &str) -> Option<String> {
    known
        .iter()
        .find(|player| player.name.eq_ignore_ascii_case(name))
        .and_then(|player| player.id.clone())
}

/// The name of the known player with XUID `xuid`.
fn known_name(known: &[Player], xuid: &str) -> Option<String> {
    known
        .iter()
        .find(|player| player.id.as_deref() == Some(xuid))
        .map(|player| player.name.clone())
}

/// Known players from the allowlist file (when it parses) and the console.
async fn known_players(
    ctx: &Context<'_>,
    permissions: &PermissionManager,
) -> Result<Vec<Player>, ApiResponse> {
    let allowlist = ctx
        .read_list(allowlist(ctx))
        .await?
        .and_then(Result::ok)
        .unwrap_or_default();
    Ok(known(&allowlist, &ctx.log_lines(permissions).await))
}

/// The XUID of a player: the given id, known players, then GeyserMC.
async fn resolve_xuid(
    name: &str,
    id: Option<String>,
    known: &[Player],
) -> Result<String, ApiResponse> {
    if let Some(id) = id {
        return Ok(id);
    }
    if let Some(xuid) = known_xuid(known, name) {
        return Ok(xuid);
    }
    lookup::geyser_xuid(name)
        .await?
        .ok_or_else(|| unprocessable("XUID unknown, let the player join once or enter the XUID"))
}

fn done() -> Result<MutationResult, ApiResponse> {
    Ok(MutationResult {
        method: Method::File,
        restart_required: false,
    })
}

#[async_trait::async_trait]
impl Game for MinecraftBedrock {
    fn id(&self) -> &'static str {
        ID
    }

    fn family(&self) -> &'static str {
        FAMILY
    }

    fn detect(&self, files: &[&str], hints: &[&str]) -> Option<u8> {
        detect(files, hints)
    }

    async fn descriptor(&self, ctx: &Context<'_>) -> Result<Descriptor, ApiResponse> {
        Ok(descriptor(ctx.state))
    }

    async fn overview(
        &self,
        ctx: &Context<'_>,
        permissions: &PermissionManager,
    ) -> Result<Contents, ApiResponse> {
        let allowlist_file = allowlist(ctx);
        let (content, allowlist, permission_entries) = tokio::try_join!(
            ctx.read_text_lossy(PROPERTIES),
            ctx.read_list(allowlist_file),
            ctx.read_list(PERMISSIONS),
        )?;
        let mut contents = Contents::default();
        if let Some(content) = content {
            contents.info = Info {
                whitelist_enabled: properties::get_bool(&content, "allow-list")
                    .or_else(|| properties::get_bool(&content, "white-list")),
                max_players: properties::get_u32(&content, "max-players"),
                online_mode: properties::get_bool(&content, "online-mode"),
            };
        }
        let allowlist = lists::shown(allowlist_file, allowlist, &mut contents.errors);
        let permission_entries =
            lists::shown(PERMISSIONS, permission_entries, &mut contents.errors);
        let known = known(&allowlist, &ctx.log_lines(permissions).await);

        let whitelist: Vec<Entry> = lists::objects(&allowlist)
            .filter_map(|entry| {
                Some(Entry {
                    name: Some(lists::string(entry, "name")?.to_string()),
                    id: lists::text(entry, "xuid").filter(|xuid| XUID.is_match(xuid)),
                    bypasses_player_limit: Some(
                        entry
                            .get("ignoresPlayerLimit")
                            .and_then(Value::as_bool)
                            .unwrap_or(false),
                    ),
                    ..Entry::default()
                })
            })
            .collect();
        let operators: Vec<Entry> = lists::objects(&permission_entries)
            .map(|entry| {
                let id = lists::text(entry, "xuid");
                Entry {
                    name: id.as_deref().and_then(|xuid| known_name(&known, xuid)),
                    id,
                    level: Some(
                        lists::string(entry, "permission")
                            .unwrap_or("member")
                            .to_string(),
                    ),
                    ..Entry::default()
                }
            })
            .collect();
        contents.lists = BTreeMap::from([
            (ListKind::Whitelist, whitelist),
            (ListKind::Operators, operators),
        ]);
        contents.known = known;
        Ok(contents)
    }

    async fn online(&self, ctx: &Context<'_>, actor: &Actor<'_>) -> Result<Online, ApiResponse> {
        let mut online = super::online(ctx, actor, &["list"]).await?;
        let known = known_players(ctx, actor.permissions).await?;
        for player in online
            .players
            .iter_mut()
            .filter(|player| player.id.is_none())
        {
            player.id = known_xuid(&known, &player.name);
        }
        Ok(online)
    }

    async fn add(
        &self,
        ctx: &Context<'_>,
        actor: &Actor<'_>,
        _method: Method,
        spec: &ListSpec,
        add: Add,
    ) -> Result<MutationResult, ApiResponse> {
        let Subject::Player { name, id } = add.subject else {
            return Err(list_not_supported());
        };
        match spec.kind {
            ListKind::Whitelist => {
                let file = allowlist(ctx);
                let mut entries = ctx.load_list(file).await?;
                let xuid = match id {
                    Some(id) => Some(id),
                    None => known_xuid(
                        &known(&entries, &ctx.log_lines(actor.permissions).await),
                        &name,
                    ),
                };
                lists::upsert(
                    &mut entries,
                    |entry| {
                        lists::has_name(entry, "name", &name)
                            || xuid.as_deref().is_some_and(|xuid| xuid_is(entry, xuid))
                    },
                    |entry| {
                        let ignores = add
                            .bypasses_player_limit
                            .or_else(|| entry.get("ignoresPlayerLimit").and_then(Value::as_bool))
                            .unwrap_or(false);
                        entry.insert("ignoresPlayerLimit".into(), Value::Bool(ignores));
                        entry.insert("name".into(), Value::from(name.as_str()));
                        if let Some(xuid) = &xuid {
                            entry.insert("xuid".into(), Value::from(xuid.as_str()));
                        }
                    },
                );
                ctx.write_list(file, &entries, actor).await?;
                ctx.command_if_running(actor, allowlist_reload(ctx)).await?;
            }
            ListKind::Operators => {
                let level = match add.level {
                    Some(level) => level,
                    None => spec
                        .levels
                        .map_or(LEVELS[0], |levels| levels.default)
                        .to_string(),
                };
                let known = known_players(ctx, actor.permissions).await?;
                let xuid = resolve_xuid(&name, id, &known).await?;
                let mut entries = ctx.load_list(PERMISSIONS).await?;
                lists::upsert(
                    &mut entries,
                    |entry| xuid_is(entry, &xuid),
                    |entry| {
                        entry.insert("permission".into(), Value::from(level));
                        entry.insert("xuid".into(), Value::from(xuid.as_str()));
                    },
                );
                ctx.write_list(PERMISSIONS, &entries, actor).await?;
                ctx.command_if_running(actor, "permission reload").await?;
            }
            ListKind::Bans | ListKind::IpBans => return Err(list_not_supported()),
        }
        done()
    }

    async fn remove(
        &self,
        ctx: &Context<'_>,
        actor: &Actor<'_>,
        _method: Method,
        spec: &ListSpec,
        selector: Selector,
    ) -> Result<MutationResult, ApiResponse> {
        let Selector::Player { name, id } = selector else {
            return Err(list_not_supported());
        };
        match spec.kind {
            ListKind::Whitelist => {
                let file = allowlist(ctx);
                let mut entries = ctx.load_list(file).await?;
                let removed = lists::remove(&mut entries, |entry| {
                    name.as_deref()
                        .is_some_and(|name| lists::has_name(entry, "name", name))
                        || id.as_deref().is_some_and(|xuid| xuid_is(entry, xuid))
                });
                if removed == 0 {
                    return Err(player_not_found());
                }
                ctx.write_list(file, &entries, actor).await?;
                ctx.command_if_running(actor, allowlist_reload(ctx)).await?;
            }
            ListKind::Operators => {
                let xuid = match (id, name) {
                    (Some(id), _) => id,
                    (None, Some(name)) => {
                        let known = known_players(ctx, actor.permissions).await?;
                        resolve_xuid(&name, None, &known).await?
                    }
                    (None, None) => return Err(ApiResponse::error("a name or an id is required")),
                };
                let mut entries = ctx.load_list(PERMISSIONS).await?;
                if lists::remove(&mut entries, |entry| xuid_is(entry, &xuid)) == 0 {
                    return Err(player_not_found());
                }
                ctx.write_list(PERMISSIONS, &entries, actor).await?;
                ctx.command_if_running(actor, "permission reload").await?;
            }
            ListKind::Bans | ListKind::IpBans => return Err(list_not_supported()),
        }
        done()
    }

    async fn set_whitelist(
        &self,
        ctx: &Context<'_>,
        actor: &Actor<'_>,
        _method: Method,
        enabled: bool,
    ) -> Result<MutationResult, ApiResponse> {
        let content = ctx.read_text(PROPERTIES).await?.unwrap_or_default();
        let key = if !properties::has(&content, "allow-list")
            && properties::has(&content, "white-list")
        {
            "white-list"
        } else {
            "allow-list"
        };
        let updated = properties::set(&content, key, if enabled { "true" } else { "false" });
        if updated != content {
            ctx.write(PROPERTIES, updated, actor).await?;
        }
        // a running server only reads server.properties on start
        Ok(MutationResult {
            method: Method::File,
            restart_required: ctx.state == ServerState::Running,
        })
    }

    async fn kick(
        &self,
        ctx: &Context<'_>,
        actor: &Actor<'_>,
        name: &str,
        reason: Option<&str>,
    ) -> Result<MutationResult, ApiResponse> {
        super::kick(ctx, actor, name, reason).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{console::split_lines, model::Blocked};

    #[test]
    fn known_players_prefer_newer_joins() {
        let allowlist = lists::parse(
            r#"[{"name":"Alex","xuid":"1"},{"name":"NoXuid"},{"name":"Some Guy","xuid":2}]"#,
        )
        .unwrap();
        let log = split_lines(
            "[2026-01-01 12:00:00:000 INFO] Player connected: alex, xuid: 3\nPlayer connected: Steve, xuid: 4",
        );
        let known = known(&allowlist, &log);
        let player = |name: &str, id: &str| Player {
            name: name.into(),
            id: Some(id.into()),
        };
        assert_eq!(
            known,
            vec![
                player("alex", "3"),
                player("Some Guy", "2"),
                player("Steve", "4")
            ]
        );
        assert_eq!(known_xuid(&known, "ALEX").as_deref(), Some("3"));
        assert_eq!(known_xuid(&known, "NoXuid"), None);
        assert_eq!(known_name(&known, "2").as_deref(), Some("Some Guy"));
    }

    #[test]
    fn running_servers_also_need_the_console() {
        let descriptor = descriptor(ServerState::Running);
        let edit = descriptor.edit.unwrap();
        assert_eq!(edit.method, Method::File);
        assert_eq!(edit.access.requires, ["files.create", "control.console"]);
        assert_eq!(edit.access.visible_with, ["files.create"]);
        assert_eq!(edit.access.blocked, None);
        let toggle = descriptor.whitelist_toggle.unwrap();
        assert_eq!(toggle.access.requires, ["files.create"]);
        assert_eq!(descriptor.online.unwrap().blocked, None);
    }

    #[test]
    fn stopped_servers_need_files_only() {
        let offline = descriptor(ServerState::Offline);
        let edit = offline.edit.unwrap();
        assert_eq!(edit.access.requires, ["files.create"]);
        assert_eq!(edit.access.blocked, None);
        assert_eq!(
            offline.kick.unwrap().access.blocked,
            Some(Blocked::NotRunning)
        );
        let stopping = descriptor(ServerState::Stopping);
        assert_eq!(
            stopping.edit.unwrap().access.blocked,
            Some(Blocked::Transition)
        );
        assert_eq!(
            stopping.whitelist_toggle.unwrap().access.blocked,
            Some(Blocked::Transition)
        );
    }

    #[test]
    fn lists_take_ids_levels_and_the_player_limit_flag() {
        for state in [ServerState::Running, ServerState::Offline] {
            let lists = descriptor(state).lists;
            let kinds: Vec<_> = lists.iter().map(|spec| spec.kind).collect();
            assert_eq!(kinds, [ListKind::Whitelist, ListKind::Operators]);
            assert!(
                lists
                    .iter()
                    .all(|spec| spec.id == IdField::Optional && !spec.reason)
            );
            assert!(lists[0].bypasses_player_limit);
            assert_eq!(lists[0].levels, None);
            assert!(!lists[1].bypasses_player_limit);
            assert_eq!(
                lists[1].levels,
                Some(Levels {
                    options: &["operator", "member", "visitor"],
                    default: "operator"
                })
            );
        }
    }
}
