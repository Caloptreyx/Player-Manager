import { prettifyId, type StatFormat } from '../lib/profiles.ts';
import type { HighlightIcon, ProfileUi, StatHighlight } from './index.ts';

// Item icons come from mc.nerothe.com, which serves the rendered inventory icon of every vanilla item and block
// item of a game version by id (`/img/<version>/minecraft_<path>.png`, a small PNG; 404 for unknown ids). Checked
// with real requests: diamond_sword, grass_block, oak_stairs, chest, shield, player_head, leaf_litter and
// copper_golem_statue all resolve for 1.26.2 (the newest version it has). Also tried: minecraft-api.vercel.app
// (503), misode/mcmeta's atlas (raw textures only, no rendered block items, 2 MB sprite) and the wiki's
// Invicon files (keyed by English names, not ids). Older or newer ids it lacks and modded items get the
// fallback tile.
const ICON_VERSION = '1.26.2';
const ICON_ID = /^minecraft:([a-z0-9_]+)$/;

const itemIcon = (id: string): string | null => {
  const path = ICON_ID.exec(id.includes(':') ? id : `minecraft:${id}`)?.[1];
  return path ? `https://mc.nerothe.com/img/${ICON_VERSION}/minecraft_${path}.png` : null;
};

/** The stats categories in the order of the game's statistics screen, then the general ones. */
const CATEGORIES = [
  'mined',
  'crafted',
  'used',
  'broken',
  'picked_up',
  'dropped',
  'killed',
  'killed_by',
  'custom',
] as const;
type Category = (typeof CATEGORIES)[number];

const DIMENSIONS = ['overworld', 'the_nether', 'the_end'] as const;
type Dimension = (typeof DIMENSIONS)[number];

// general stats counted in ticks; the movement stats end in `_one_cm`, the damage stats in tenths of a health point
const TICK_STATS: Record<string, true> = {
  play_time: true,
  play_one_minute: true,
  time_since_death: true,
  time_since_rest: true,
  sneak_time: true,
  total_world_time: true,
};

const statFormat = (category: string, key: string): StatFormat => {
  if (category !== 'minecraft:custom') return 'count';
  const path = key.replace(/^minecraft:/, '');
  if (TICK_STATS[path] === true) return 'ticks';
  if (path.endsWith('_one_cm')) return 'centimeters';
  return path.startsWith('damage_') ? 'tenthHealth' : 'count';
};

const HIGHLIGHT_ICONS = {
  playTime: 'time',
  deaths: 'death',
  mobKills: 'mobKill',
  playerKills: 'playerKill',
  walked: 'walk',
  sprinted: 'run',
  flown: 'fly',
  jumps: 'jump',
  damageDealt: 'attack',
  damageTaken: 'defense',
} satisfies Record<string, HighlightIcon>;

/** The key stats over all categories; missing stats count as zero. */
const statHighlights: ProfileUi['statHighlights'] = (t, stats) => {
  const custom = stats['minecraft:custom'] ?? {};
  const sum = (...keys: string[]) => keys.reduce((total, key) => total + (custom[`minecraft:${key}`] ?? 0), 0);
  const highlight = (key: keyof typeof HIGHLIGHT_ICONS, value: number, format: StatFormat): StatHighlight => ({
    key,
    label: t(`games.minecraftJava.stats.${key}`, {}),
    icon: HIGHLIGHT_ICONS[key],
    value,
    format,
  });

  return [
    // renamed from play_one_minute (which counted ticks all along) in 1.17
    highlight('playTime', custom['minecraft:play_time'] ?? custom['minecraft:play_one_minute'] ?? 0, 'ticks'),
    highlight('deaths', sum('deaths'), 'count'),
    highlight('mobKills', sum('mob_kills'), 'count'),
    highlight('playerKills', sum('player_kills'), 'count'),
    highlight('walked', sum('walk_one_cm'), 'centimeters'),
    highlight('sprinted', sum('sprint_one_cm'), 'centimeters'),
    // creative flight and elytra gliding
    highlight('flown', sum('fly_one_cm', 'aviate_one_cm'), 'centimeters'),
    highlight('jumps', sum('jump'), 'count'),
    highlight('damageDealt', sum('damage_dealt'), 'tenthHealth'),
    highlight('damageTaken', sum('damage_taken'), 'tenthHealth'),
  ];
};

const isCategory = (path: string): path is Category => (CATEGORIES as readonly string[]).includes(path);
const isDimension = (path: string): path is Dimension => (DIMENSIONS as readonly string[]).includes(path);

const minecraftJavaProfile: ProfileUi = {
  itemIcon,
  // mc-heads renders the skin of a uuid (undashed) as a full body turned to the right
  bodyUrl: (player, size) => {
    const key = player.id ? player.id.replaceAll('-', '').toLowerCase() : player.name;
    return key ? `https://mc-heads.net/body/${encodeURIComponent(key)}/${size}/right` : null;
  },
  dimensionLabel: (t, id) => {
    const path = id.replace(/^minecraft:/, '');
    return isDimension(path) ? t(`games.minecraftJava.dimensions.${path}`, {}) : prettifyId(id);
  },
  statHighlights,
  statCategories: (t, present) => {
    const known = CATEGORIES.map((category) => `minecraft:${category}`);
    return [...known.filter((id) => present.includes(id)), ...present.filter((id) => !known.includes(id)).sort()].map(
      (id) => {
        const path = id.replace(/^minecraft:/, '');
        return { id, label: isCategory(path) ? t(`games.minecraftJava.statCategories.${path}`, {}) : prettifyId(id) };
      },
    );
  },
  statFormat,
  // the movement stats read better without their unit suffix, which the value shows
  statLabel: (key) => prettifyId(key.replace(/_one_cm$/, '').replace(/^minecraft:play_one_minute$/, 'play_time')),
  // blocks and items show their icon, mobs their spawn egg
  statIcon: (category, key) => {
    if (category === 'minecraft:custom') return null;
    return category === 'minecraft:killed' || category === 'minecraft:killed_by'
      ? itemIcon(`${key}_spawn_egg`)
      : itemIcon(key);
  },
};

export default minecraftJavaProfile;
