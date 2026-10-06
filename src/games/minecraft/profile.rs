//! Java player profiles: the player data file `<uuid>.dat` (NBT) with its statistics and
//! advancements JSON, read into a [`Profile`]; actions edit the NBT of offline players and
//! send commands for online ones. See [`Dirs`] for where worlds keep these files.
//! Handles the item formats before and after 1.20.5 and the equipment layouts before and
//! after 1.21.5.
use super::{
    CONSOLE, FILES, PROPERTIES, READ_FILES,
    java::{USERCACHE, usercache_players},
    java_id, live,
    nbt::{self, Compound, List, Tag},
    properties,
};
use crate::{
    console::strip_ansi,
    context::{Actor, Context, conflict, profile_not_found, unprocessable},
    games::{ProfileAction, ProfileMode},
    model::{
        Advancement, Advancements, Armor, Blocked, Capability, Container, ENDER_CHEST_SLOTS,
        Effect, Enchantment, Gamemode, Item, Method, MutationResult, Position, Profile,
        ProfileSummary, ProfilesSpec, ServerState, Slot,
    },
};
use serde_json::Value;
use shared::response::ApiResponse;
use std::collections::{BTreeMap, HashMap};
use wings_api::DirectoryEntry;

const DEFAULT_WORLD: &str = "world";
const OVERWORLD: &str = "minecraft:overworld";
const DEFAULT_MAX_HEALTH: f64 = 20.0;
const AIR: &str = "minecraft:air";
/// Recipe unlocks are stored as advancements too.
const RECIPES: &str = "minecraft:recipes/";
/// The `equipment` compound of 1.21.5+ players.
const EQUIPMENT: [(Slot, &str); 5] = [
    (Slot::Armor(Armor::Head), "head"),
    (Slot::Armor(Armor::Chest), "chest"),
    (Slot::Armor(Armor::Legs), "legs"),
    (Slot::Armor(Armor::Feet), "feet"),
    (Slot::Offhand, "offhand"),
];
/// Effect ids before 1.20.2 stored them as numbers.
const LEGACY_EFFECTS: [&str; 33] = [
    "speed",
    "slowness",
    "haste",
    "mining_fatigue",
    "strength",
    "instant_health",
    "instant_damage",
    "jump_boost",
    "nausea",
    "regeneration",
    "resistance",
    "fire_resistance",
    "water_breathing",
    "invisibility",
    "blindness",
    "night_vision",
    "hunger",
    "weakness",
    "poison",
    "wither",
    "health_boost",
    "absorption",
    "saturation",
    "glowing",
    "levitation",
    "luck",
    "unluck",
    "slow_falling",
    "conduit_power",
    "dolphins_grace",
    "bad_omen",
    "hero_of_the_village",
    "darkness",
];

/// Viewing needs the files, offline edits write them, live actions are commands.
pub fn spec(state: ServerState) -> ProfilesSpec {
    ProfilesSpec {
        view: Capability {
            requires: READ_FILES,
            visible_with: READ_FILES,
            blocked: None,
        },
        edit_offline: Some(Capability {
            requires: FILES,
            visible_with: FILES,
            blocked: super::transition(state),
        }),
        edit_live: Some(Capability {
            requires: CONSOLE,
            visible_with: CONSOLE,
            blocked: (state != ServerState::Running).then_some(Blocked::NotRunning),
        }),
    }
}

/// The world folder: `level-name`, else `world`.
fn world(content: Option<&str>) -> &str {
    content
        .and_then(|content| properties::get(content, "level-name"))
        .filter(|name| !name.is_empty())
        .unwrap_or(DEFAULT_WORLD)
}

/// Where a world keeps its player files: `players/data`, `players/stats` and
/// `players/advancements` since Minecraft 26.1, `playerdata`, `stats` and `advancements`
/// before.
#[derive(Debug, PartialEq, Eq)]
struct Dirs {
    data: String,
    stats: String,
    advancements: String,
}

impl Dirs {
    /// Both layouts of `world`, the current one first.
    fn candidates(world: &str) -> [Self; 2] {
        [
            Self {
                data: format!("{world}/players/data"),
                stats: format!("{world}/players/stats"),
                advancements: format!("{world}/players/advancements"),
            },
            Self {
                data: format!("{world}/playerdata"),
                stats: format!("{world}/stats"),
                advancements: format!("{world}/advancements"),
            },
        ]
    }

    /// The layout `world` uses with the entries of its player data directory; `None` when it
    /// has neither (no player has joined yet).
    async fn locate(
        ctx: &Context<'_>,
        world: &str,
    ) -> Result<Option<(Self, Vec<DirectoryEntry>)>, ApiResponse> {
        for dirs in Self::candidates(world) {
            if let Some(entries) = ctx.list(&dirs.data).await? {
                return Ok(Some((dirs, entries)));
            }
        }
        Ok(None)
    }

    /// The layout that holds `file` in its player data directory, with the file's entry.
    async fn find(
        ctx: &Context<'_>,
        world: &str,
        file: &str,
    ) -> Result<Option<(Self, DirectoryEntry)>, ApiResponse> {
        for dirs in Self::candidates(world) {
            if let Some(entry) = ctx.stat(&dirs.data, file).await? {
                return Ok(Some((dirs, entry)));
            }
        }
        Ok(None)
    }
}

/// The player id of a player data file: only `<dashed lowercase uuid>.dat` (not `.dat_old`,
/// temporary or other files).
fn file_id(name: &str) -> Option<&str> {
    let stem = name.strip_suffix(".dat")?;
    (java_id(stem).as_deref() == Some(stem)).then_some(stem)
}

fn rfc3339(time: chrono::DateTime<chrono::Local>) -> String {
    time.with_timezone(&chrono::Utc)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

/// Names by UUID from `usercache.json` (unreadable caches give none).
fn usercache_names(usercache: Option<Result<Vec<Value>, String>>) -> HashMap<String, String> {
    let entries = usercache.and_then(Result::ok).unwrap_or_default();
    usercache_players(&entries)
        .into_iter()
        .filter_map(|player| Some((player.id?, player.name)))
        .collect()
}

/// Every player data file, newest first.
pub async fn list(ctx: &Context<'_>) -> Result<Vec<ProfileSummary>, ApiResponse> {
    let (content, usercache) =
        tokio::try_join!(ctx.read_text_lossy(PROPERTIES), ctx.read_list(USERCACHE))?;
    let world = world(content.as_deref());
    let entries = Dirs::locate(ctx, world)
        .await?
        .map(|(_, entries)| entries)
        .unwrap_or_default();
    let names = usercache_names(usercache);
    let mut profiles: Vec<ProfileSummary> = entries
        .iter()
        .filter(|entry| !entry.directory)
        .filter_map(|entry| {
            let id = file_id(&entry.name)?;
            Some(ProfileSummary {
                id: id.to_string(),
                name: names.get(id).cloned(),
                last_saved: rfc3339(entry.modified),
            })
        })
        .collect();
    // RFC 3339 in UTC with whole seconds sorts as text
    profiles.sort_by(|a, b| b.last_saved.cmp(&a.last_saved));
    Ok(profiles)
}

/// The root tag of a player data file (422 when it does not read). Minecraft gzips the file;
/// Wings hands gzip files out decompressed, so both forms are accepted.
fn decode(data: &[u8]) -> Result<(String, Compound), ApiResponse> {
    nbt::decompress(data)
        .and_then(|raw| nbt::read(&raw))
        .map_err(|err| unprocessable(format!("could not read the player data: {err}")))
}

/// The profile of player `id` (dashed lowercase UUID).
pub async fn load(ctx: &Context<'_>, id: &str) -> Result<Profile, ApiResponse> {
    let content = ctx.read_text_lossy(PROPERTIES).await?;
    let world = world(content.as_deref());
    let file = format!("{id}.dat");
    let (dirs, entry) = Dirs::find(ctx, world, &file)
        .await?
        .ok_or_else(profile_not_found)?;
    let data_path = format!("{}/{file}", dirs.data);
    let stats_path = format!("{}/{id}.json", dirs.stats);
    let advancements_path = format!("{}/{id}.json", dirs.advancements);
    let (data, stats, advancements, usercache) = tokio::try_join!(
        ctx.read_file(&data_path),
        ctx.read_file(&stats_path),
        ctx.read_file(&advancements_path),
        ctx.read_list(USERCACHE),
    )?;
    let (_, root) = decode(&data.ok_or_else(profile_not_found)?)?;
    Ok(profile(
        id,
        usercache_names(usercache).remove(id),
        rfc3339(entry.modified),
        &root,
        stats.as_deref(),
        advancements.as_deref(),
    ))
}

/// Runs an action: commands for an online player, an edit of the saved data otherwise.
pub async fn act(
    ctx: &Context<'_>,
    actor: &Actor<'_>,
    id: &str,
    mode: ProfileMode,
    action: ProfileAction,
) -> Result<MutationResult, ApiResponse> {
    let name = match mode {
        ProfileMode::Live { name } => name,
        ProfileMode::File => return edit_file(ctx, actor, id, &action).await,
    };
    let commands = commands(&name, &action);
    let commands: Vec<&str> = commands.iter().map(String::as_str).collect();
    live::run(ctx, actor, &commands).await
}

async fn edit_file(
    ctx: &Context<'_>,
    actor: &Actor<'_>,
    id: &str,
    action: &ProfileAction,
) -> Result<MutationResult, ApiResponse> {
    let content = ctx.read_text_lossy(PROPERTIES).await?;
    let file = format!("{id}.dat");
    let (dirs, _) = Dirs::find(ctx, world(content.as_deref()), &file)
        .await?
        .ok_or_else(profile_not_found)?;
    let path = format!("{}/{file}", dirs.data);
    let data = ctx.read_file(&path).await?.ok_or_else(profile_not_found)?;
    let (name, mut root) = decode(&data)?;
    if edit(&mut root, action).map_err(conflict)? {
        let raw = nbt::write(&name, &root)
            .map_err(|err| unprocessable(format!("could not write the player data: {err}")))?;
        ctx.write_file(&path, nbt::gzip(&raw), actor).await?;
    }
    Ok(MutationResult {
        method: Method::File,
        restart_required: false,
        message: None,
    })
}

fn gamemode(id: i64) -> Option<Gamemode> {
    match id {
        0 => Some(Gamemode::Survival),
        1 => Some(Gamemode::Creative),
        2 => Some(Gamemode::Adventure),
        3 => Some(Gamemode::Spectator),
        _ => None,
    }
}

fn gamemode_id(gamemode: Gamemode) -> i32 {
    match gamemode {
        Gamemode::Survival => 0,
        Gamemode::Creative => 1,
        Gamemode::Adventure => 2,
        Gamemode::Spectator => 3,
    }
}

fn gamemode_name(gamemode: Gamemode) -> &'static str {
    match gamemode {
        Gamemode::Survival => "survival",
        Gamemode::Creative => "creative",
        Gamemode::Adventure => "adventure",
        Gamemode::Spectator => "spectator",
    }
}

/// The commands of a live action for the online player `name`.
fn commands(name: &str, action: &ProfileAction) -> Vec<String> {
    let clear = |slot: Slot| format!("item replace entity {name} {slot} with {AIR}");
    match action {
        ProfileAction::ClearSlot(slot) => vec![clear(*slot)],
        ProfileAction::ClearContainer(Container::Inventory) => vec![format!("clear {name}")],
        ProfileAction::ClearContainer(Container::EnderChest) => (0..ENDER_CHEST_SLOTS)
            .map(|index| clear(Slot::EnderChest(index)))
            .collect(),
        ProfileAction::SetGamemode(gamemode) => {
            vec![format!("gamemode {} {name}", gamemode_name(*gamemode))]
        }
        ProfileAction::SetXpLevel(level) => vec![
            format!("xp set {name} {level} levels"),
            format!("xp set {name} 0 points"),
        ],
        ProfileAction::Give { item, count } => vec![format!("give {name} {item} {count}")],
    }
}

/// The slot of an `Inventory` entry: 0-8 hotbar, 9-35 inventory, and before 1.21.5 100-103
/// armor (feet to head) and -106 offhand.
fn inventory_slot(number: i64) -> Option<Slot> {
    Some(match number {
        0..=8 => Slot::Hotbar(number as u8),
        9..=35 => Slot::Inventory((number - 9) as u8),
        100 => Slot::Armor(Armor::Feet),
        101 => Slot::Armor(Armor::Legs),
        102 => Slot::Armor(Armor::Chest),
        103 => Slot::Armor(Armor::Head),
        -106 => Slot::Offhand,
        _ => return None,
    })
}

/// The `Inventory` entry number of a slot (`None` for the ender chest).
fn inventory_number(slot: Slot) -> Option<i64> {
    Some(match slot {
        Slot::Hotbar(index) => index.into(),
        Slot::Inventory(index) => i64::from(index) + 9,
        Slot::Armor(Armor::Feet) => 100,
        Slot::Armor(Armor::Legs) => 101,
        Slot::Armor(Armor::Chest) => 102,
        Slot::Armor(Armor::Head) => 103,
        Slot::Offhand => -106,
        Slot::EnderChest(_) => return None,
    })
}

fn equipment_key(slot: Slot) -> Option<&'static str> {
    EQUIPMENT
        .iter()
        .find(|(equipped, _)| *equipped == slot)
        .map(|(_, key)| *key)
}

/// The compounds of the list `key`.
fn compounds<'a>(parent: &'a Compound, key: &str) -> impl Iterator<Item = &'a Compound> {
    parent
        .get(key)
        .and_then(Tag::as_list)
        .map(List::items)
        .unwrap_or_default()
        .iter()
        .filter_map(Tag::as_compound)
}

/// An id stored as text, or as a number in old saves.
fn id_text(tag: &Tag) -> Option<String> {
    match tag {
        Tag::String(id) => Some(id.clone()),
        tag => tag.as_i64().map(|id| id.to_string()),
    }
}

fn push_json_text(output: &mut String, value: &Value) {
    match value {
        Value::String(text) => output.push_str(text),
        Value::Number(number) => output.push_str(&number.to_string()),
        Value::Bool(flag) => output.push_str(if *flag { "true" } else { "false" }),
        Value::Array(parts) => {
            for part in parts {
                push_json_text(output, part);
            }
        }
        Value::Object(component) => {
            if let Some(text) = ["text", "fallback", "translate", "keybind", "selector"]
                .iter()
                .find_map(|key| component.get(*key)?.as_str())
            {
                output.push_str(text);
            }
            if let Some(Value::Array(extra)) = component.get("extra") {
                for part in extra {
                    push_json_text(output, part);
                }
            }
        }
        Value::Null => {}
    }
}

fn push_nbt_text(output: &mut String, tag: &Tag) {
    match tag {
        Tag::String(text) => output.push_str(text),
        Tag::List(parts) => {
            for part in parts.items() {
                push_nbt_text(output, part);
            }
        }
        Tag::Compound(component) => {
            if let Some(text) = ["text", "fallback", "translate", "keybind", "selector"]
                .iter()
                .find_map(|key| component.get(key)?.as_str())
            {
                output.push_str(text);
            }
            if let Some(Tag::List(extra)) = component.get("extra") {
                for part in extra.items() {
                    push_nbt_text(output, part);
                }
            }
        }
        _ => {}
    }
}

/// A text component as plain text: JSON in a string (before 1.21.5), NBT (1.21.5+), or a
/// legacy string with `§` codes; `None` when empty.
fn plain_text(tag: &Tag) -> Option<String> {
    let mut text = String::new();
    match tag {
        Tag::String(value) => {
            let json = value
                .trim_start()
                .starts_with(['{', '[', '"'])
                .then(|| serde_json::from_str::<Value>(value).ok())
                .flatten();
            match json {
                Some(json) => push_json_text(&mut text, &json),
                None => text = strip_ansi(value),
            }
        }
        tag => push_nbt_text(&mut text, tag),
    }
    (!text.is_empty()).then_some(text)
}

/// `minecraft:enchantments`: `{levels: {id: level}}` before 1.21.5, a plain map after.
fn component_enchantments(enchantments: &Compound) -> Vec<Enchantment> {
    let levels = enchantments
        .get("levels")
        .and_then(Tag::as_compound)
        .unwrap_or(enchantments);
    levels
        .iter()
        .filter_map(|(id, level)| {
            Some(Enchantment {
                id: id.to_string(),
                level: level.as_i64()?,
            })
        })
        .collect()
}

/// `tag.Enchantments`: `[{id, lvl}]` before 1.20.5.
fn legacy_enchantments(tag: &Compound) -> Vec<Enchantment> {
    compounds(tag, "Enchantments")
        .filter_map(|enchantment| {
            Some(Enchantment {
                id: id_text(enchantment.get("id")?)?,
                level: enchantment.get("lvl")?.as_i64()?,
            })
        })
        .collect()
}

/// An item stack in either format: 1.20.5+ (`count`, `components`) or older (`Count`, `tag`);
/// `None` for empty stacks.
fn item(slot: Slot, stack: &Compound) -> Option<Item> {
    let id = id_text(stack.get("id")?)?;
    if id == AIR || id == "air" {
        return None;
    }
    let count = stack
        .get("count")
        .or_else(|| stack.get("Count"))
        .map_or(Some(1), Tag::as_i64)?;
    if count <= 0 {
        return None;
    }
    let components = stack.get("components").and_then(Tag::as_compound);
    let tag = stack.get("tag").and_then(Tag::as_compound);
    let component = |key: &str| components.and_then(|components| components.get(key));
    let name = component("minecraft:custom_name")
        .or_else(|| tag?.get("display")?.as_compound()?.get("Name"))
        .and_then(plain_text);
    let enchantments = match (component("minecraft:enchantments"), tag) {
        (Some(enchantments), _) => enchantments
            .as_compound()
            .map(component_enchantments)
            .unwrap_or_default(),
        (None, Some(tag)) => legacy_enchantments(tag),
        (None, None) => Vec::new(),
    };
    let damage = component("minecraft:damage")
        .or_else(|| tag?.get("Damage"))
        .and_then(Tag::as_i64);
    Some(Item {
        slot: slot.to_string(),
        id,
        count,
        name,
        enchantments,
        damage,
        snbt: stack.to_string(),
    })
}

/// Inventory, hotbar, armor and offhand, in slot order.
fn inventory(root: &Compound) -> Vec<Item> {
    let mut items: Vec<(Slot, Item)> = compounds(root, "Inventory")
        .filter_map(|stack| {
            let slot = inventory_slot(stack.get("Slot")?.as_i64()?)?;
            Some((slot, item(slot, stack)?))
        })
        .collect();
    if let Some(equipment) = root.get("equipment").and_then(Tag::as_compound) {
        for (slot, key) in EQUIPMENT {
            if let Some(item) = equipment
                .get(key)
                .and_then(Tag::as_compound)
                .and_then(|stack| item(slot, stack))
            {
                items.push((slot, item));
            }
        }
    }
    items.sort_by_key(|(slot, _)| *slot);
    items.into_iter().map(|(_, item)| item).collect()
}

fn ender_chest(root: &Compound) -> Vec<Item> {
    let mut items: Vec<(Slot, Item)> = compounds(root, "EnderItems")
        .filter_map(|stack| {
            let index = u8::try_from(stack.get("Slot")?.as_i64()?)
                .ok()
                .filter(|&index| index < ENDER_CHEST_SLOTS)?;
            let slot = Slot::EnderChest(index);
            Some((slot, item(slot, stack)?))
        })
        .collect();
    items.sort_by_key(|(slot, _)| *slot);
    items.into_iter().map(|(_, item)| item).collect()
}

/// A dimension id; saves before 1.16 store -1, 0 or 1.
fn dimension(tag: Option<&Tag>) -> String {
    match tag {
        Some(Tag::String(id)) => id.clone(),
        Some(tag) => match tag.as_i64() {
            Some(-1) => "minecraft:the_nether",
            Some(1) => "minecraft:the_end",
            _ => OVERWORLD,
        }
        .to_string(),
        None => OVERWORLD.to_string(),
    }
}

fn position(root: &Compound) -> Option<Position> {
    let [x, y, z] = root.get("Pos")?.as_list()?.items() else {
        return None;
    };
    Some(Position {
        x: x.as_f64()?,
        y: y.as_f64()?,
        z: z.as_f64()?,
        dimension: dimension(root.get("Dimension")),
    })
}

/// The respawn point: a `respawn` compound (1.21.5+) or `SpawnX/Y/Z` and `SpawnDimension`.
fn spawn(root: &Compound) -> Option<Position> {
    if let Some(respawn) = root.get("respawn").and_then(Tag::as_compound) {
        let Some(Tag::IntArray(position)) = respawn.get("pos") else {
            return None;
        };
        let &[x, y, z] = position.as_slice() else {
            return None;
        };
        return Some(Position {
            x: x.into(),
            y: y.into(),
            z: z.into(),
            dimension: dimension(respawn.get("dimension")),
        });
    }
    let coordinate = |key: &str| root.get(key).and_then(Tag::as_f64);
    Some(Position {
        x: coordinate("SpawnX")?,
        y: coordinate("SpawnY")?,
        z: coordinate("SpawnZ")?,
        dimension: dimension(root.get("SpawnDimension")),
    })
}

/// Active effects: `active_effects` (1.20.2+) or `ActiveEffects` with numeric ids.
fn effects(root: &Compound) -> Vec<Effect> {
    let key = if root.get("active_effects").is_some() {
        "active_effects"
    } else {
        "ActiveEffects"
    };
    compounds(root, key)
        .filter_map(|effect| {
            let field = |new: &str, old: &str| effect.get(new).or_else(|| effect.get(old));
            let id = match field("id", "Id")? {
                Tag::String(id) => id.clone(),
                tag => {
                    let number = tag.as_i64()?;
                    usize::try_from(number - 1)
                        .ok()
                        .and_then(|index| LEGACY_EFFECTS.get(index))
                        .map_or_else(|| number.to_string(), |name| format!("minecraft:{name}"))
                }
            };
            // the amplifier byte is unsigned
            let amplifier = match field("amplifier", "Amplifier") {
                Some(&Tag::Byte(amplifier)) => i64::from(amplifier as u8),
                other => other.and_then(Tag::as_i64).unwrap_or(0),
            };
            Some(Effect {
                id,
                amplifier,
                duration: field("duration", "Duration")
                    .and_then(Tag::as_i64)
                    .unwrap_or(0),
            })
        })
        .collect()
}

/// The base of the max health attribute (`attributes` since 1.20.5, `Attributes` before).
fn max_health(root: &Compound) -> f64 {
    let key = if root.get("attributes").is_some() {
        "attributes"
    } else {
        "Attributes"
    };
    compounds(root, key)
        .find_map(|attribute| {
            let id = attribute
                .get("id")
                .or_else(|| attribute.get("Name"))?
                .as_str()?;
            let id = id.strip_prefix("minecraft:").unwrap_or(id);
            if !matches!(
                id,
                "max_health" | "generic.max_health" | "generic.maxHealth"
            ) {
                return None;
            }
            attribute
                .get("base")
                .or_else(|| attribute.get("Base"))?
                .as_f64()
        })
        .unwrap_or(DEFAULT_MAX_HEALTH)
}

/// `{ stats: { category: { key: value } } }`; `None` when it does not parse.
fn stats(json: &[u8]) -> Option<BTreeMap<String, BTreeMap<String, i64>>> {
    let file: Value = serde_json::from_slice(json).ok()?;
    let categories = file.get("stats")?.as_object()?;
    Some(
        categories
            .iter()
            .filter_map(|(category, values)| {
                let values: BTreeMap<String, i64> = values
                    .as_object()?
                    .iter()
                    .filter_map(|(key, value)| Some((key.clone(), value.as_i64()?)))
                    .collect();
                Some((category.clone(), values))
            })
            .collect(),
    )
}

fn timestamp(text: &str) -> Option<chrono::DateTime<chrono::FixedOffset>> {
    chrono::DateTime::parse_from_str(text, "%Y-%m-%d %H:%M:%S %z").ok()
}

/// Done advancements without recipes, newest first; each done at its latest criterion.
fn advancements(json: &[u8]) -> Option<Advancements> {
    let file: Value = serde_json::from_slice(json).ok()?;
    let mut done: Vec<(Option<chrono::DateTime<chrono::FixedOffset>>, Advancement)> = file
        .as_object()?
        .iter()
        .filter(|(id, progress)| {
            !id.starts_with(RECIPES) && progress.get("done").and_then(Value::as_bool) == Some(true)
        })
        .map(|(id, progress)| {
            let latest = progress
                .get("criteria")
                .and_then(Value::as_object)
                .into_iter()
                .flat_map(|criteria| criteria.values())
                .filter_map(Value::as_str)
                .max_by_key(|&time| (timestamp(time), time));
            (
                latest.and_then(timestamp),
                Advancement {
                    id: id.clone(),
                    done_at: latest.map(str::to_string),
                },
            )
        })
        .collect();
    done.sort_by(|(a_time, a), (b_time, b)| b_time.cmp(a_time).then_with(|| a.id.cmp(&b.id)));
    Some(Advancements {
        done: done.len() as u32,
        items: done
            .into_iter()
            .map(|(_, advancement)| advancement)
            .collect(),
    })
}

/// The profile of player data `root`; `name` from `usercache.json`, else the name Paper and
/// Spigot keep in the data.
fn profile(
    id: &str,
    name: Option<String>,
    last_saved: String,
    root: &Compound,
    stats_file: Option<&[u8]>,
    advancements_file: Option<&[u8]>,
) -> Profile {
    let int = |key: &str| root.get(key).and_then(Tag::as_i64);
    // floats widen to f64 exactly, so narrowing back is lossless
    let float = |key: &str| {
        root.get(key)
            .and_then(Tag::as_f64)
            .map(|value| value as f32)
    };
    let last_known_name = || {
        root.get("bukkit")?
            .as_compound()?
            .get("lastKnownName")?
            .as_str()
            .map(str::to_string)
    };
    Profile {
        id: id.to_string(),
        name: name.or_else(last_known_name),
        last_saved,
        data_version: int("DataVersion"),
        gamemode: int("playerGameType").and_then(gamemode),
        health: float("Health"),
        max_health: Some(max_health(root)),
        food: int("foodLevel"),
        saturation: float("foodSaturationLevel"),
        xp_level: int("XpLevel"),
        xp_progress: float("XpP"),
        xp_total: int("XpTotal"),
        position: position(root),
        spawn: spawn(root),
        effects: effects(root),
        inventory: inventory(root),
        ender_chest: ender_chest(root),
        stats: stats_file.and_then(stats),
        advancements: advancements_file.and_then(advancements),
    }
}

/// Removes the entries of the list `key` in `slot`; whether there were any.
fn remove_slot(root: &mut Compound, key: &str, slot: i64) -> bool {
    match root.get_mut(key) {
        Some(Tag::List(list)) => {
            list.retain(|entry| {
                entry
                    .as_compound()
                    .and_then(|entry| entry.get("Slot"))
                    .and_then(Tag::as_i64)
                    != Some(slot)
            }) > 0
        }
        _ => false,
    }
}

fn clear_list(root: &mut Compound, key: &str) -> bool {
    match root.get_mut(key) {
        Some(Tag::List(list)) => list.retain(|_| false) > 0,
        _ => false,
    }
}

fn equipment(root: &mut Compound) -> Option<&mut Compound> {
    match root.get_mut("equipment") {
        Some(Tag::Compound(equipment)) => Some(equipment),
        _ => None,
    }
}

/// Sets `key` to `tag`; whether it changed.
fn set(root: &mut Compound, key: &str, tag: Tag) -> bool {
    if root.get(key) == Some(&tag) {
        return false;
    }
    root.insert(key, tag);
    true
}

/// Applies an action to saved player data; whether anything changed. Everything else stays
/// as it was.
fn edit(root: &mut Compound, action: &ProfileAction) -> Result<bool, &'static str> {
    Ok(match *action {
        ProfileAction::ClearSlot(Slot::EnderChest(index)) => {
            remove_slot(root, "EnderItems", index.into())
        }
        ProfileAction::ClearSlot(slot) => {
            let listed =
                inventory_number(slot).is_some_and(|number| remove_slot(root, "Inventory", number));
            let equipped = equipment_key(slot).is_some_and(|key| {
                equipment(root).is_some_and(|equipment| equipment.remove(key).is_some())
            });
            listed | equipped
        }
        ProfileAction::ClearContainer(Container::Inventory) => {
            let mut changed = clear_list(root, "Inventory");
            if let Some(equipment) = equipment(root) {
                for (_, key) in EQUIPMENT {
                    changed |= equipment.remove(key).is_some();
                }
            }
            changed
        }
        ProfileAction::ClearContainer(Container::EnderChest) => clear_list(root, "EnderItems"),
        ProfileAction::SetGamemode(gamemode) => {
            set(root, "playerGameType", Tag::Int(gamemode_id(gamemode)))
        }
        ProfileAction::SetXpLevel(level) => {
            let level = i32::try_from(level).map_err(|_| "the level is too high")?;
            let leveled = set(root, "XpLevel", Tag::Int(level));
            let reset = set(root, "XpP", Tag::Float(0.0));
            leveled | reset
        }
        ProfileAction::Give { .. } => return Err("the player must be online"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn compound(entries: Vec<(&str, Tag)>) -> Compound {
        let mut compound = Compound::default();
        for (key, tag) in entries {
            compound.insert(key, tag);
        }
        compound
    }

    fn string(value: &str) -> Tag {
        Tag::String(value.into())
    }

    fn list(items: Vec<Tag>) -> Tag {
        Tag::List(List::from(items))
    }

    /// An item as 1.20.5+ saves it in `Inventory`.
    fn modern_stack(slot: i8, id: &str, count: i32, components: Vec<(&str, Tag)>) -> Tag {
        let mut stack = vec![
            ("Slot", Tag::Byte(slot)),
            ("id", string(id)),
            ("count", Tag::Int(count)),
        ];
        if !components.is_empty() {
            stack.push(("components", Tag::Compound(compound(components))));
        }
        Tag::Compound(compound(stack))
    }

    /// An item as 1.20.4 and older save it.
    fn legacy_stack(slot: i8, id: &str, count: i8, tag: Option<Compound>) -> Tag {
        let mut stack = vec![
            ("Slot", Tag::Byte(slot)),
            ("id", string(id)),
            ("Count", Tag::Byte(count)),
        ];
        if let Some(tag) = tag {
            stack.push(("tag", Tag::Compound(tag)));
        }
        Tag::Compound(compound(stack))
    }

    /// A 1.21.5+ player: `equipment` holds armor and offhand.
    fn modern_player() -> Compound {
        compound(vec![
            ("DataVersion", Tag::Int(4325)),
            ("playerGameType", Tag::Int(1)),
            ("Health", Tag::Float(17.5)),
            ("foodLevel", Tag::Int(18)),
            ("foodSaturationLevel", Tag::Float(2.5)),
            ("XpLevel", Tag::Int(30)),
            ("XpP", Tag::Float(0.25)),
            ("XpTotal", Tag::Int(1395)),
            (
                "Pos",
                list(vec![
                    Tag::Double(10.5),
                    Tag::Double(64.0),
                    Tag::Double(-3.25),
                ]),
            ),
            ("Dimension", string("minecraft:the_nether")),
            (
                "respawn",
                Tag::Compound(compound(vec![
                    ("pos", Tag::IntArray(vec![1, 70, -2])),
                    ("dimension", string("minecraft:overworld")),
                ])),
            ),
            (
                "attributes",
                list(vec![Tag::Compound(compound(vec![
                    ("id", string("minecraft:max_health")),
                    ("base", Tag::Double(40.0)),
                ]))]),
            ),
            (
                "active_effects",
                list(vec![Tag::Compound(compound(vec![
                    ("id", string("minecraft:speed")),
                    ("amplifier", Tag::Byte(1)),
                    ("duration", Tag::Int(-1)),
                ]))]),
            ),
            (
                "Inventory",
                list(vec![
                    modern_stack(
                        0,
                        "minecraft:diamond_sword",
                        1,
                        vec![
                            (
                                "minecraft:custom_name",
                                Tag::Compound(compound(vec![
                                    ("text", string("Ex")),
                                    ("extra", list(vec![string("calibur")])),
                                ])),
                            ),
                            (
                                "minecraft:enchantments",
                                Tag::Compound(compound(vec![("minecraft:sharpness", Tag::Int(5))])),
                            ),
                            ("minecraft:damage", Tag::Int(12)),
                        ],
                    ),
                    modern_stack(9, "minecraft:torch", 64, vec![]),
                    modern_stack(35, "minecraft:dirt", 3, vec![]),
                ]),
            ),
            (
                "equipment",
                Tag::Compound(compound(vec![
                    (
                        "head",
                        Tag::Compound(compound(vec![
                            ("id", string("minecraft:turtle_helmet")),
                            ("count", Tag::Int(1)),
                        ])),
                    ),
                    (
                        "offhand",
                        Tag::Compound(compound(vec![("id", string("minecraft:shield"))])),
                    ),
                ])),
            ),
            (
                "EnderItems",
                list(vec![modern_stack(26, "minecraft:elytra", 1, vec![])]),
            ),
        ])
    }

    /// A 1.20.4 Paper player: armor and offhand are `Inventory` slots 100-103 and -106.
    fn legacy_player() -> Compound {
        compound(vec![
            ("DataVersion", Tag::Int(3700)),
            ("playerGameType", Tag::Int(0)),
            (
                "Pos",
                list(vec![Tag::Double(0.5), Tag::Double(70.0), Tag::Double(0.5)]),
            ),
            ("Dimension", string("minecraft:overworld")),
            ("SpawnX", Tag::Int(5)),
            ("SpawnY", Tag::Int(64)),
            ("SpawnZ", Tag::Int(6)),
            (
                "Attributes",
                list(vec![Tag::Compound(compound(vec![
                    ("Name", string("minecraft:generic.max_health")),
                    ("Base", Tag::Double(20.0)),
                ]))]),
            ),
            (
                "ActiveEffects",
                list(vec![Tag::Compound(compound(vec![
                    ("Id", Tag::Byte(16)),
                    ("Amplifier", Tag::Byte(-1)),
                    ("Duration", Tag::Int(600)),
                ]))]),
            ),
            (
                "Inventory",
                list(vec![
                    legacy_stack(
                        4,
                        "minecraft:diamond_pickaxe",
                        1,
                        Some(compound(vec![
                            ("Damage", Tag::Int(7)),
                            (
                                "display",
                                Tag::Compound(compound(vec![(
                                    "Name",
                                    string(r#"{"text":"Digger","italic":false}"#),
                                )])),
                            ),
                            (
                                "Enchantments",
                                list(vec![Tag::Compound(compound(vec![
                                    ("id", string("minecraft:efficiency")),
                                    ("lvl", Tag::Short(4)),
                                ]))]),
                            ),
                        ])),
                    ),
                    legacy_stack(100, "minecraft:iron_boots", 1, None),
                    legacy_stack(103, "minecraft:iron_helmet", 1, None),
                    legacy_stack(-106, "minecraft:torch", 16, None),
                    legacy_stack(80, "minecraft:stone", 1, None),
                    legacy_stack(5, "minecraft:air", 0, None),
                ]),
            ),
            (
                "bukkit",
                Tag::Compound(compound(vec![("lastKnownName", string("Steve"))])),
            ),
        ])
    }

    fn summary(items: &[Item]) -> Vec<(&str, &str, i64)> {
        items
            .iter()
            .map(|item| (item.slot.as_str(), item.id.as_str(), item.count))
            .collect()
    }

    #[test]
    fn reads_modern_players() {
        let root = modern_player();
        let profile = profile("id", None, "2026-10-06T12:00:00Z".into(), &root, None, None);
        assert_eq!(profile.data_version, Some(4325));
        assert_eq!(profile.gamemode, Some(Gamemode::Creative));
        assert_eq!(
            (profile.health, profile.max_health, profile.food),
            (Some(17.5), Some(40.0), Some(18))
        );
        assert_eq!(
            (profile.xp_level, profile.xp_progress, profile.xp_total),
            (Some(30), Some(0.25), Some(1395))
        );
        assert_eq!(
            profile.position,
            Some(Position {
                x: 10.5,
                y: 64.0,
                z: -3.25,
                dimension: "minecraft:the_nether".into(),
            })
        );
        assert_eq!(
            profile.spawn,
            Some(Position {
                x: 1.0,
                y: 70.0,
                z: -2.0,
                dimension: OVERWORLD.into(),
            })
        );
        assert_eq!(
            profile.effects,
            [Effect {
                id: "minecraft:speed".into(),
                amplifier: 1,
                duration: -1,
            }]
        );
        assert_eq!(
            summary(&profile.inventory),
            [
                ("hotbar.0", "minecraft:diamond_sword", 1),
                ("inventory.0", "minecraft:torch", 64),
                ("inventory.26", "minecraft:dirt", 3),
                ("armor.head", "minecraft:turtle_helmet", 1),
                ("weapon.offhand", "minecraft:shield", 1),
            ]
        );
        let sword = &profile.inventory[0];
        assert_eq!(sword.name.as_deref(), Some("Excalibur"));
        assert_eq!(
            sword.enchantments,
            [Enchantment {
                id: "minecraft:sharpness".into(),
                level: 5,
            }]
        );
        assert_eq!(sword.damage, Some(12));
        assert_eq!(
            sword.snbt,
            r#"{Slot:0b,id:"minecraft:diamond_sword",count:1,components:{"minecraft:custom_name":{text:"Ex",extra:["calibur"]},"minecraft:enchantments":{"minecraft:sharpness":5},"minecraft:damage":12}}"#
        );
        assert_eq!(
            summary(&profile.ender_chest),
            [("enderchest.26", "minecraft:elytra", 1)]
        );
        assert_eq!((profile.stats, profile.advancements), (None, None));
    }

    #[test]
    fn reads_legacy_players() {
        let root = legacy_player();
        let profile = profile("id", None, String::new(), &root, None, None);
        assert_eq!(profile.name.as_deref(), Some("Steve"));
        assert_eq!(profile.gamemode, Some(Gamemode::Survival));
        assert_eq!(profile.max_health, Some(20.0));
        assert_eq!(
            profile.spawn,
            Some(Position {
                x: 5.0,
                y: 64.0,
                z: 6.0,
                dimension: OVERWORLD.into(),
            })
        );
        assert_eq!(
            profile.effects,
            [Effect {
                id: "minecraft:night_vision".into(),
                amplifier: 255,
                duration: 600,
            }]
        );
        // slot 80 is no slot a player has, air is no item
        assert_eq!(
            summary(&profile.inventory),
            [
                ("hotbar.4", "minecraft:diamond_pickaxe", 1),
                ("armor.head", "minecraft:iron_helmet", 1),
                ("armor.feet", "minecraft:iron_boots", 1),
                ("weapon.offhand", "minecraft:torch", 16),
            ]
        );
        let pickaxe = &profile.inventory[0];
        assert_eq!(pickaxe.name.as_deref(), Some("Digger"));
        assert_eq!(
            pickaxe.enchantments,
            [Enchantment {
                id: "minecraft:efficiency".into(),
                level: 4,
            }]
        );
        assert_eq!(pickaxe.damage, Some(7));
        assert!(profile.ender_chest.is_empty());
        // usercache names win over the saved one
        let named = super::profile("id", Some("Alex".into()), String::new(), &root, None, None);
        assert_eq!(named.name.as_deref(), Some("Alex"));
    }

    #[test]
    fn flattens_names_and_enchantment_maps() {
        assert_eq!(
            plain_text(&string(
                r#"[{"text":"A"},{"text":"B","extra":[{"translate":"x"}]}]"#
            ))
            .as_deref(),
            Some("ABx")
        );
        assert_eq!(
            plain_text(&string(r#""Quoted""#)).as_deref(),
            Some("Quoted")
        );
        assert_eq!(plain_text(&string("§6Gold")).as_deref(), Some("Gold"));
        assert_eq!(
            plain_text(&string("{not json")).as_deref(),
            Some("{not json")
        );
        assert_eq!(plain_text(&string("")), None);
        // 1.20.5 to 1.21.4: `{levels: {...}, show_in_tooltip}`
        let wrapped = compound(vec![
            (
                "levels",
                Tag::Compound(compound(vec![
                    ("minecraft:unbreaking", Tag::Int(3)),
                    ("minecraft:mending", Tag::Int(1)),
                ])),
            ),
            ("show_in_tooltip", Tag::Byte(0)),
        ]);
        assert_eq!(
            component_enchantments(&wrapped),
            [
                Enchantment {
                    id: "minecraft:unbreaking".into(),
                    level: 3,
                },
                Enchantment {
                    id: "minecraft:mending".into(),
                    level: 1,
                },
            ]
        );
    }

    #[test]
    fn reads_stats_and_advancements() {
        let stats = stats(
            br#"{"stats":{"minecraft:custom":{"minecraft:play_time":72000,"minecraft:jump":12}},"DataVersion":4325}"#,
        )
        .unwrap();
        assert_eq!(stats["minecraft:custom"]["minecraft:play_time"], 72000);
        assert_eq!(stats["minecraft:custom"]["minecraft:jump"], 12);

        let advancements = advancements(
            br#"{
                "minecraft:story/root": {"criteria": {"crafting_table": "2026-01-01 10:00:00 +0000"}, "done": true},
                "minecraft:story/mine_stone": {"criteria": {"get_stone": "2026-01-01 12:30:00 +0200", "x": "2026-01-01 09:00:00 +0000"}, "done": true},
                "minecraft:story/smelt_iron": {"criteria": {}, "done": false},
                "minecraft:recipes/misc/torch": {"criteria": {"has_coal": "2026-01-02 00:00:00 +0000"}, "done": true},
                "DataVersion": 4325
            }"#,
        )
        .unwrap();
        assert_eq!(advancements.done, 2);
        assert_eq!(
            advancements.items,
            [
                Advancement {
                    id: "minecraft:story/mine_stone".into(),
                    done_at: Some("2026-01-01 12:30:00 +0200".into()),
                },
                Advancement {
                    id: "minecraft:story/root".into(),
                    done_at: Some("2026-01-01 10:00:00 +0000".into()),
                },
            ]
        );
    }

    #[test]
    fn names_player_data_files() {
        let id = "069a79f4-44e9-4726-a5be-fca90e38aaf5";
        assert_eq!(file_id(&format!("{id}.dat")), Some(id));
        assert_eq!(file_id(&format!("{id}.dat_old")), None);
        assert_eq!(file_id(&format!("{id}.dat.tmp")), None);
        assert_eq!(file_id(&format!("{}.dat", id.to_uppercase())), None);
        assert_eq!(file_id("069a79f444e94726a5befca90e38aaf5.dat"), None);
        assert_eq!(file_id("level.dat"), None);
        assert_eq!(world(None), "world");
        assert_eq!(world(Some("level-name=survival\n")), "survival");
    }

    #[test]
    fn maps_slots_both_ways() {
        for number in (0..=35).chain([100, 101, 102, 103, -106]) {
            let slot = inventory_slot(number).unwrap();
            assert_eq!(inventory_number(slot), Some(number));
        }
        assert_eq!(inventory_slot(36), None);
        assert_eq!(inventory_slot(-105), None);
        assert_eq!(inventory_slot(20).unwrap().to_string(), "inventory.11");
        assert_eq!(inventory_slot(102).unwrap().to_string(), "armor.chest");
        assert_eq!(inventory_number(Slot::EnderChest(0)), None);
        assert_eq!(equipment_key(Slot::Offhand), Some("offhand"));
        assert_eq!(equipment_key(Slot::Hotbar(0)), None);
    }

    #[test]
    fn builds_live_commands() {
        let run = |action| commands("Steve", &action);
        assert_eq!(
            run(ProfileAction::ClearSlot(Slot::Armor(Armor::Head))),
            ["item replace entity Steve armor.head with minecraft:air"]
        );
        assert_eq!(
            run(ProfileAction::ClearContainer(Container::Inventory)),
            ["clear Steve"]
        );
        let ender = run(ProfileAction::ClearContainer(Container::EnderChest));
        assert_eq!(ender.len(), 27);
        assert_eq!(
            ender[26],
            "item replace entity Steve enderchest.26 with minecraft:air"
        );
        assert_eq!(
            run(ProfileAction::SetGamemode(Gamemode::Spectator)),
            ["gamemode spectator Steve"]
        );
        assert_eq!(
            run(ProfileAction::SetXpLevel(30)),
            ["xp set Steve 30 levels", "xp set Steve 0 points"]
        );
        assert_eq!(
            run(ProfileAction::Give {
                item: "minecraft:diamond".into(),
                count: 64,
            }),
            ["give Steve minecraft:diamond 64"]
        );
    }

    /// Writes, edits, then reads back like a saved file, so edits must survive encoding.
    fn edited(root: &Compound, action: ProfileAction) -> (bool, Compound) {
        let mut root = nbt::read(&nbt::write("", root).unwrap()).unwrap().1;
        let changed = edit(&mut root, &action).unwrap();
        (
            changed,
            nbt::read(&nbt::write("", &root).unwrap()).unwrap().1,
        )
    }

    #[test]
    fn clears_slots_in_both_layouts() {
        let (changed, root) = edited(&modern_player(), ProfileAction::ClearSlot(Slot::Hotbar(0)));
        assert!(changed);
        assert_eq!(
            summary(&inventory(&root))[0],
            ("inventory.0", "minecraft:torch", 64)
        );
        let (changed, root) = edited(&modern_player(), ProfileAction::ClearSlot(Slot::Offhand));
        assert!(changed);
        assert!(
            inventory(&root)
                .iter()
                .all(|item| item.slot != "weapon.offhand")
        );
        let (changed, root) = edited(&legacy_player(), ProfileAction::ClearSlot(Slot::Offhand));
        assert!(changed);
        assert!(
            inventory(&root)
                .iter()
                .all(|item| item.slot != "weapon.offhand")
        );
        let (changed, root) = edited(
            &modern_player(),
            ProfileAction::ClearSlot(Slot::EnderChest(26)),
        );
        assert!(changed && ender_chest(&root).is_empty());
        let (changed, root) = edited(&modern_player(), ProfileAction::ClearSlot(Slot::Hotbar(8)));
        assert!(!changed);
        assert_eq!(root, modern_player());
    }

    #[test]
    fn clears_containers_and_keeps_the_rest() {
        let (changed, root) = edited(
            &modern_player(),
            ProfileAction::ClearContainer(Container::Inventory),
        );
        assert!(changed);
        assert!(inventory(&root).is_empty());
        assert_eq!(ender_chest(&root).len(), 1);
        let mut expected = modern_player();
        expected.insert("Inventory", list(vec![]));
        expected.insert("equipment", Tag::Compound(Compound::default()));
        assert_eq!(root, expected);

        let (changed, root) = edited(
            &legacy_player(),
            ProfileAction::ClearContainer(Container::EnderChest),
        );
        assert!(!changed);
        assert_eq!(inventory(&root).len(), 4);
    }

    #[test]
    fn sets_gamemode_and_level() {
        let (changed, root) = edited(
            &legacy_player(),
            ProfileAction::SetGamemode(Gamemode::Adventure),
        );
        assert!(changed);
        assert_eq!(root.get("playerGameType"), Some(&Tag::Int(2)));
        let (changed, root) = edited(&modern_player(), ProfileAction::SetXpLevel(7));
        assert!(changed);
        assert_eq!(root.get("XpLevel"), Some(&Tag::Int(7)));
        assert_eq!(root.get("XpP"), Some(&Tag::Float(0.0)));
        assert_eq!(root.get("XpTotal"), Some(&Tag::Int(1395)));
        let mut root = modern_player();
        assert_eq!(
            edit(
                &mut root,
                &ProfileAction::Give {
                    item: "minecraft:stone".into(),
                    count: 1,
                }
            ),
            Err("the player must be online")
        );
    }
}
