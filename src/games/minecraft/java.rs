//! Minecraft: Java Edition (vanilla, Paper/Spigot, Fabric, Forge...). A running server is
//! changed through console commands (it rewrites its JSON files itself), a stopped one by
//! editing the files.
use super::{
    CONSOLE, FAMILY, FILES, JAVA_NAME, PROPERTIES, SCORE_JAVA_FILES, SCORE_SHARED_FILES, UUID,
    java_id, live, lookup, profile, properties,
};
use crate::{
    context::{Actor, Context, list_not_supported, player_not_found},
    games::{Add, Contents, Game, ProfileAction, ProfileMode, Selector, Subject},
    lists::{self, Object},
    model::{
        Capability, Descriptor, Entry, IdField, Info, Levels, ListKind, ListSpec, ListTarget,
        Method, MethodCapability, MutationResult, Online, Player, PlayerId, PlayerName, Profile,
        ProfileSummary, ServerState,
    },
};
use axum::http::StatusCode;
use serde_json::Value;
use shared::{models::user::PermissionManager, response::ApiResponse};
use std::{collections::BTreeMap, net::IpAddr};

pub const ID: &str = "minecraft_java";

const WHITELIST: &str = "whitelist.json";
const OPS: &str = "ops.json";
const BANS: &str = "banned-players.json";
const IP_BANS: &str = "banned-ips.json";
pub(super) const USERCACHE: &str = "usercache.json";

/// Files only a Java server creates.
const OWN_FILES: [&str; 5] = [OPS, BANS, IP_BANS, USERCACHE, "eula.txt"];

/// The ban reason servers store when none is given.
const BAN_REASON: &str = "Banned by an operator.";

/// Op levels, `level` in `ops.json`.
const LEVELS: &[&str] = &["1", "2", "3", "4"];

const CONSOLE_OR_FILES: &[&str] = &["control.console", "files.create"];

pub struct MinecraftJava;

fn detect(files: &[&str]) -> Option<u8> {
    let has = |name: &str| files.contains(&name);
    if OWN_FILES.iter().any(|name| has(name))
        || files
            .iter()
            .any(|name| name.to_ascii_lowercase().ends_with(".jar"))
    {
        Some(SCORE_JAVA_FILES)
    } else if has(WHITELIST) || has(PROPERTIES) {
        Some(SCORE_SHARED_FILES)
    } else {
        None
    }
}

/// The level new ops get in file edits: `op-permission-level`, else 4.
fn default_level(properties_content: Option<&str>) -> u8 {
    properties_content
        .and_then(|content| properties::get_u32(content, "op-permission-level"))
        .filter(|level| (1..=4).contains(level))
        .unwrap_or(4) as u8
}

/// Commands while running, file edits otherwise (blocked while starting or stopping); levels,
/// ids and the player limit flag only apply to file edits.
pub(crate) fn descriptor(state: ServerState, default_level: u8) -> Descriptor {
    let files = state != ServerState::Running;
    let (requires, method) = if files {
        (FILES, Method::File)
    } else {
        (CONSOLE, Method::Command)
    };
    let access = Capability {
        requires,
        visible_with: CONSOLE_OR_FILES,
        blocked: super::transition(state),
    };
    let id = if files {
        IdField::Optional
    } else {
        IdField::None
    };
    let player = |kind| ListSpec {
        kind,
        target: ListTarget::Player,
        id,
        reason: false,
        levels: None,
        bypasses_player_limit: false,
    };
    let level = usize::from(default_level.clamp(1, 4)) - 1;

    Descriptor {
        player_name: PlayerName {
            pattern: &JAVA_NAME,
        },
        player_id: PlayerId {
            kind: "uuid",
            pattern: &UUID,
        },
        lists: vec![
            player(ListKind::Whitelist),
            ListSpec {
                levels: files.then_some(Levels {
                    options: LEVELS,
                    default: LEVELS[level],
                }),
                bypasses_player_limit: files,
                ..player(ListKind::Operators)
            },
            ListSpec {
                reason: true,
                ..player(ListKind::Bans)
            },
            ListSpec {
                kind: ListKind::IpBans,
                target: ListTarget::Ip,
                id: IdField::None,
                reason: true,
                levels: None,
                bypasses_player_limit: false,
            },
        ],
        edit: Some(MethodCapability { access, method }),
        whitelist_toggle: Some(MethodCapability { access, method }),
        online: Some(super::online_capability(state)),
        kick: Some(super::kick_capability(state)),
        profiles: Some(profile::spec(state)),
    }
}

fn file(kind: ListKind) -> &'static str {
    match kind {
        ListKind::Whitelist => WHITELIST,
        ListKind::Operators => OPS,
        ListKind::Bans => BANS,
        ListKind::IpBans => IP_BANS,
    }
}

/// Whether the entry's `uuid` is `uuid` (dashed lowercase), however it is written.
fn uuid_is(entry: &Object, uuid: &str) -> bool {
    lists::string(entry, "uuid")
        .and_then(java_id)
        .is_some_and(|found| found == uuid)
}

/// Whether the entry bans `ip`, however the address is written.
fn bans_ip(entry: &Object, ip: IpAddr) -> bool {
    lists::string(entry, "ip").is_some_and(|found| {
        found
            .trim()
            .parse::<IpAddr>()
            .is_ok_and(|found| found == ip)
    })
}

/// `created` of a new ban entry, in the server's `yyyy-MM-dd HH:mm:ss Z` format.
fn ban_created() -> String {
    chrono::Utc::now()
        .format("%Y-%m-%d %H:%M:%S +0000")
        .to_string()
}

fn with_reason(command: String, reason: Option<&str>) -> String {
    match reason {
        Some(reason) => format!("{command} {reason}"),
        None => command,
    }
}

/// Players of a `usercache.json`.
pub(super) fn usercache_players(entries: &[Value]) -> Vec<Player> {
    lists::objects(entries)
        .filter_map(|entry| {
            Some(Player {
                name: lists::string(entry, "name")?.to_string(),
                id: lists::string(entry, "uuid").and_then(java_id),
            })
        })
        .collect()
}

fn string(entry: &Object, key: &str) -> Option<String> {
    lists::string(entry, key).map(str::to_string)
}

fn player_entry(entry: &Object) -> Entry {
    Entry {
        name: string(entry, "name"),
        id: lists::string(entry, "uuid").and_then(java_id),
        ..Entry::default()
    }
}

fn operator_entry(entry: &Object) -> Entry {
    let level = entry
        .get("level")
        .and_then(Value::as_u64)
        .filter(|level| (1..=4).contains(level))
        .unwrap_or(4);
    Entry {
        level: Some(level.to_string()),
        bypasses_player_limit: Some(
            entry
                .get("bypassesPlayerLimit")
                .and_then(Value::as_bool)
                .unwrap_or(false),
        ),
        ..player_entry(entry)
    }
}

/// `entry`'s ban fields on top of `base`.
fn ban_entry(entry: &Object, base: Entry) -> Entry {
    Entry {
        reason: string(entry, "reason"),
        source: string(entry, "source"),
        created: string(entry, "created"),
        expires: string(entry, "expires"),
        ..base
    }
}

/// The Floodgate gamertag behind a prefixed name (`.Steve` → `Steve`).
fn floodgate_gamertag(name: &str) -> Option<&str> {
    name.strip_prefix(['.', '*'])
}

/// `(name, dashed uuid)` of a player for file edits: the given id, `usercache.json`, GeyserMC
/// for Floodgate names, then Mojang, or the offline UUID when `online-mode` is false.
async fn resolve(
    ctx: &Context<'_>,
    name: &str,
    id: Option<String>,
) -> Result<(String, String), ApiResponse> {
    if let Some(id) = id {
        return Ok((name.to_string(), id));
    }
    if let Some(Ok(cache)) = ctx.read_list(USERCACHE).await?
        && let Some(player) = usercache_players(&cache)
            .into_iter()
            .find(|player| player.name.eq_ignore_ascii_case(name))
        && let Some(uuid) = player.id
    {
        return Ok((player.name, uuid));
    }
    if let Some(gamertag) = floodgate_gamertag(name) {
        // Floodgate writes the spaces of gamertags as underscores
        let xuid = lookup::geyser_xuid(&gamertag.replace('_', " "))
            .await?
            .ok_or_else(player_not_found)?;
        let uuid = lookup::floodgate_uuid(&xuid).ok_or_else(player_not_found)?;
        return Ok((name.to_string(), uuid));
    }
    let online_mode = ctx
        .read_text_lossy(PROPERTIES)
        .await?
        .and_then(|content| properties::get_bool(&content, "online-mode"));
    if online_mode == Some(false) {
        return Ok((name.to_string(), lookup::offline_uuid(name)));
    }
    let (uuid, canonical) = lookup::mojang_profile(name)
        .await?
        .ok_or_else(player_not_found)?;
    Ok((canonical, uuid))
}

/// The result of a file edit.
fn edited() -> Result<MutationResult, ApiResponse> {
    Ok(MutationResult {
        method: Method::File,
        restart_required: false,
        message: None,
    })
}

#[async_trait::async_trait]
impl Game for MinecraftJava {
    fn id(&self) -> &'static str {
        ID
    }

    fn family(&self) -> &'static str {
        FAMILY
    }

    fn detect(&self, files: &[&str], _hints: &[&str]) -> Option<u8> {
        detect(files)
    }

    fn normalize_id(&self, id: &str) -> String {
        java_id(id).unwrap_or_else(|| id.to_string())
    }

    async fn descriptor(&self, ctx: &Context<'_>) -> Result<Descriptor, ApiResponse> {
        let default_level = if ctx.state == ServerState::Running {
            4
        } else {
            default_level(ctx.read_text_lossy(PROPERTIES).await?.as_deref())
        };
        Ok(descriptor(ctx.state, default_level))
    }

    async fn overview(
        &self,
        ctx: &Context<'_>,
        _permissions: &PermissionManager,
    ) -> Result<Contents, ApiResponse> {
        let (content, whitelist, ops, bans, ip_bans, usercache) = tokio::try_join!(
            ctx.read_text_lossy(PROPERTIES),
            ctx.read_list(WHITELIST),
            ctx.read_list(OPS),
            ctx.read_list(BANS),
            ctx.read_list(IP_BANS),
            ctx.read_list(USERCACHE),
        )?;
        let mut contents = Contents::default();
        if let Some(content) = content {
            contents.info = Info {
                whitelist_enabled: properties::get_bool(&content, "white-list"),
                max_players: properties::get_u32(&content, "max-players"),
                online_mode: properties::get_bool(&content, "online-mode"),
            };
        }
        let errors = &mut contents.errors;
        let whitelist: Vec<Entry> = lists::objects(&lists::shown(WHITELIST, whitelist, errors))
            .filter(|entry| lists::string(entry, "name").is_some())
            .map(player_entry)
            .collect();
        let ops: Vec<Entry> = lists::objects(&lists::shown(OPS, ops, errors))
            .map(operator_entry)
            .collect();
        let bans: Vec<Entry> = lists::objects(&lists::shown(BANS, bans, errors))
            .map(|entry| ban_entry(entry, player_entry(entry)))
            .collect();
        let ip_bans: Vec<Entry> = lists::objects(&lists::shown(IP_BANS, ip_bans, errors))
            .filter_map(|entry| {
                let ip = string(entry, "ip")?;
                Some(ban_entry(
                    entry,
                    Entry {
                        ip: Some(ip),
                        ..Entry::default()
                    },
                ))
            })
            .collect();
        contents.known = usercache_players(&lists::shown(USERCACHE, usercache, errors));
        contents.lists = BTreeMap::from([
            (ListKind::Whitelist, whitelist),
            (ListKind::Operators, ops),
            (ListKind::Bans, bans),
            (ListKind::IpBans, ip_bans),
        ]);
        Ok(contents)
    }

    async fn online(&self, ctx: &Context<'_>, actor: &Actor<'_>) -> Result<Online, ApiResponse> {
        live::java(ctx, actor).await
    }

    async fn add(
        &self,
        ctx: &Context<'_>,
        actor: &Actor<'_>,
        method: Method,
        spec: &ListSpec,
        add: Add,
    ) -> Result<MutationResult, ApiResponse> {
        let reason = add.reason.as_deref();
        let (name, id) = match add.subject {
            Subject::Ip(ip) if spec.kind == ListKind::IpBans => {
                if method == Method::Command {
                    let command = with_reason(format!("ban-ip {ip}"), reason);
                    return live::run(ctx, actor, &[command.as_str()]).await;
                }
                let created = ban_created();
                let mut entries = ctx.load_list(IP_BANS).await?;
                lists::upsert(
                    &mut entries,
                    |entry| bans_ip(entry, ip),
                    |entry| {
                        entry.insert("ip".into(), Value::from(ip.to_string()));
                        entry.insert("created".into(), Value::from(created.as_str()));
                        entry.insert("source".into(), Value::from("Server"));
                        entry.insert("expires".into(), Value::from("forever"));
                        entry.insert("reason".into(), Value::from(reason.unwrap_or(BAN_REASON)));
                    },
                );
                ctx.write_list(IP_BANS, &entries, actor).await?;
                return edited();
            }
            Subject::Player { name, id } if spec.kind != ListKind::IpBans => (name, id),
            _ => return Err(list_not_supported()),
        };

        if method == Method::Command {
            let command = match spec.kind {
                ListKind::Whitelist => format!("whitelist add {name}"),
                ListKind::Operators => format!("op {name}"),
                ListKind::Bans | ListKind::IpBans => with_reason(format!("ban {name}"), reason),
            };
            return live::run(ctx, actor, &[command.as_str()]).await;
        }

        let (name, uuid) = resolve(ctx, &name, id).await?;
        let default_level = spec
            .levels
            .and_then(|levels| levels.default.parse::<u64>().ok())
            .unwrap_or(4);
        let level = add.level.and_then(|level| level.parse::<u64>().ok());
        let file = file(spec.kind);
        let mut entries = ctx.load_list(file).await?;
        lists::upsert(
            &mut entries,
            |entry| uuid_is(entry, &uuid) || lists::has_name(entry, "name", &name),
            |entry| {
                let level = level
                    .or_else(|| {
                        entry
                            .get("level")
                            .and_then(Value::as_u64)
                            .filter(|level| (1..=4).contains(level))
                    })
                    .unwrap_or(default_level);
                let bypasses = add
                    .bypasses_player_limit
                    .or_else(|| entry.get("bypassesPlayerLimit").and_then(Value::as_bool))
                    .unwrap_or(false);
                entry.insert("uuid".into(), Value::from(uuid.as_str()));
                entry.insert("name".into(), Value::from(name.as_str()));
                match spec.kind {
                    ListKind::Operators => {
                        entry.insert("level".into(), Value::from(level));
                        entry.insert("bypassesPlayerLimit".into(), Value::Bool(bypasses));
                    }
                    ListKind::Bans => {
                        entry.insert("created".into(), Value::from(ban_created()));
                        entry.insert("source".into(), Value::from("Server"));
                        entry.insert("expires".into(), Value::from("forever"));
                        entry.insert("reason".into(), Value::from(reason.unwrap_or(BAN_REASON)));
                    }
                    ListKind::Whitelist | ListKind::IpBans => {}
                }
            },
        );
        ctx.write_list(file, &entries, actor).await?;
        edited()
    }

    async fn remove(
        &self,
        ctx: &Context<'_>,
        actor: &Actor<'_>,
        method: Method,
        spec: &ListSpec,
        selector: Selector,
    ) -> Result<MutationResult, ApiResponse> {
        let (name, id) = match selector {
            Selector::Ip(ip) if spec.kind == ListKind::IpBans => {
                if method == Method::Command {
                    return live::run(ctx, actor, &[format!("pardon-ip {ip}").as_str()]).await;
                }
                let mut entries = ctx.load_list(IP_BANS).await?;
                if lists::remove(&mut entries, |entry| bans_ip(entry, ip)) == 0 {
                    return Err(
                        ApiResponse::error("IP ban not found").with_status(StatusCode::NOT_FOUND)
                    );
                }
                ctx.write_list(IP_BANS, &entries, actor).await?;
                return edited();
            }
            Selector::Player { name, id } if spec.kind != ListKind::IpBans => (name, id),
            _ => return Err(list_not_supported()),
        };
        let file = file(spec.kind);

        if method == Method::Command {
            let name = match (name, &id) {
                (Some(name), _) => name,
                (None, Some(uuid)) => {
                    let entries = ctx.load_list(file).await?;
                    lists::objects(&entries)
                        .find(|entry| uuid_is(entry, uuid))
                        .and_then(|entry| lists::string(entry, "name"))
                        .filter(|name| JAVA_NAME.is_match(name))
                        .map(str::to_string)
                        .ok_or_else(player_not_found)?
                }
                (None, None) => return Err(ApiResponse::error("a name or an id is required")),
            };
            let command = match spec.kind {
                ListKind::Whitelist => "whitelist remove",
                ListKind::Operators => "deop",
                ListKind::Bans | ListKind::IpBans => "pardon",
            };
            return live::run(ctx, actor, &[format!("{command} {name}").as_str()]).await;
        }

        let mut entries = ctx.load_list(file).await?;
        let removed = lists::remove(&mut entries, |entry| {
            id.as_deref().is_some_and(|uuid| uuid_is(entry, uuid))
                || name
                    .as_deref()
                    .is_some_and(|name| lists::has_name(entry, "name", name))
        });
        if removed == 0 {
            return Err(player_not_found());
        }
        ctx.write_list(file, &entries, actor).await?;
        edited()
    }

    async fn set_whitelist(
        &self,
        ctx: &Context<'_>,
        actor: &Actor<'_>,
        method: Method,
        enabled: bool,
    ) -> Result<MutationResult, ApiResponse> {
        if method == Method::Command {
            let command = if enabled {
                "whitelist on"
            } else {
                "whitelist off"
            };
            return live::run(ctx, actor, &[command]).await;
        }

        let content = ctx.read_text(PROPERTIES).await?.unwrap_or_default();
        let updated = properties::set(
            &content,
            "white-list",
            if enabled { "true" } else { "false" },
        );
        if updated != content {
            ctx.write(PROPERTIES, updated, actor).await?;
        }
        edited()
    }

    /// Over RCON when usable, like every Java command.
    async fn kick(
        &self,
        ctx: &Context<'_>,
        actor: &Actor<'_>,
        name: &str,
        reason: Option<&str>,
    ) -> Result<MutationResult, ApiResponse> {
        live::run(ctx, actor, &[super::kick_command(name, reason).as_str()]).await
    }

    async fn profiles(&self, ctx: &Context<'_>) -> Result<Vec<ProfileSummary>, ApiResponse> {
        profile::list(ctx).await
    }

    async fn profile(&self, ctx: &Context<'_>, id: &str) -> Result<Profile, ApiResponse> {
        profile::load(ctx, id).await
    }

    async fn profile_action(
        &self,
        ctx: &Context<'_>,
        actor: &Actor<'_>,
        id: &str,
        mode: ProfileMode,
        action: ProfileAction,
    ) -> Result<MutationResult, ApiResponse> {
        profile::act(ctx, actor, id, mode, action).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::Blocked;

    fn spec(descriptor: &Descriptor, kind: ListKind) -> ListSpec {
        *descriptor
            .lists
            .iter()
            .find(|spec| spec.kind == kind)
            .unwrap()
    }

    #[test]
    fn running_servers_use_commands() {
        let descriptor = descriptor(ServerState::Running, 2);
        let edit = descriptor.edit.unwrap();
        assert_eq!(edit.method, Method::Command);
        assert_eq!(edit.access.requires, ["control.console"]);
        assert_eq!(edit.access.blocked, None);
        assert_eq!(descriptor.whitelist_toggle, Some(edit));
        let kinds: Vec<_> = descriptor.lists.iter().map(|spec| spec.kind).collect();
        assert_eq!(
            kinds,
            [
                ListKind::Whitelist,
                ListKind::Operators,
                ListKind::Bans,
                ListKind::IpBans
            ]
        );
        let operators = spec(&descriptor, ListKind::Operators);
        assert_eq!(operators.id, IdField::None);
        assert_eq!(operators.levels, None);
        assert!(!operators.bypasses_player_limit);
        assert!(spec(&descriptor, ListKind::Bans).reason);
        assert_eq!(spec(&descriptor, ListKind::IpBans).target, ListTarget::Ip);
        assert_eq!(descriptor.online.unwrap().blocked, None);
        assert_eq!(descriptor.kick.unwrap().access.blocked, None);
    }

    #[test]
    fn stopped_servers_edit_files() {
        let descriptor = descriptor(ServerState::Offline, 2);
        let edit = descriptor.edit.unwrap();
        assert_eq!(edit.method, Method::File);
        assert_eq!(edit.access.requires, ["files.create"]);
        assert_eq!(
            edit.access.visible_with,
            ["control.console", "files.create"]
        );
        assert_eq!(edit.access.blocked, None);
        let operators = spec(&descriptor, ListKind::Operators);
        assert_eq!(operators.id, IdField::Optional);
        assert_eq!(
            operators.levels,
            Some(Levels {
                options: LEVELS,
                default: "2"
            })
        );
        assert!(operators.bypasses_player_limit);
        assert_eq!(spec(&descriptor, ListKind::Whitelist).id, IdField::Optional);
        assert_eq!(spec(&descriptor, ListKind::IpBans).id, IdField::None);
        assert_eq!(
            descriptor.online.unwrap().blocked,
            Some(Blocked::NotRunning)
        );
        assert_eq!(
            descriptor.kick.unwrap().access.blocked,
            Some(Blocked::NotRunning)
        );
    }

    #[test]
    fn transitions_block_edits() {
        for state in [ServerState::Starting, ServerState::Stopping] {
            let descriptor = descriptor(state, 4);
            let edit = descriptor.edit.unwrap();
            assert_eq!(edit.access.blocked, Some(Blocked::Transition));
            assert_eq!(edit.method, Method::File);
            assert_eq!(edit.access.requires, ["files.create"]);
            assert_eq!(
                descriptor.online.unwrap().blocked,
                Some(Blocked::Transition)
            );
            assert_eq!(
                descriptor.kick.unwrap().access.blocked,
                Some(Blocked::NotRunning)
            );
        }
    }

    #[test]
    fn default_op_level() {
        assert_eq!(default_level(None), 4);
        assert_eq!(default_level(Some("op-permission-level=2\n")), 2);
        assert_eq!(default_level(Some("op-permission-level=7\n")), 4);
        assert_eq!(default_level(Some("op-permission-level=x\n")), 4);
    }

    #[test]
    fn normalizes_uuids() {
        assert_eq!(
            MinecraftJava.normalize_id("069A79F444E94726A5BEFCA90E38AAF5"),
            "069a79f4-44e9-4726-a5be-fca90e38aaf5"
        );
    }
}
