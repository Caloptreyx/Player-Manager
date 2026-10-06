import type { Access } from './access.ts';
import type { Item, OnlinePlayers, Profile, ServerState } from './model.ts';
import { type PlayerRef, sameName } from './players.ts';

// pure helpers of the player profile view: slot layout, value formatting and how actions run

// ---- slots: the command slot names of the contract, in the order the grids show them

const slotRange = (prefix: string, count: number) => Array.from({ length: count }, (_, index) => `${prefix}.${index}`);

export const ARMOR_SLOTS = ['armor.head', 'armor.chest', 'armor.legs', 'armor.feet'] as const;
export const OFFHAND_SLOT = 'weapon.offhand';
/** The 27 storage slots above the hotbar, row by row. */
export const MAIN_SLOTS = slotRange('inventory', 27);
export const HOTBAR_SLOTS = slotRange('hotbar', 9);
export const ENDER_CHEST_SLOTS = slotRange('enderchest', 27);

const INVENTORY_LAYOUT = new Set<string>([...ARMOR_SLOTS, OFFHAND_SLOT, ...MAIN_SLOTS, ...HOTBAR_SLOTS]);
const ENDER_CHEST_LAYOUT = new Set<string>(ENDER_CHEST_SLOTS);

export interface SlotItems {
  bySlot: Map<string, Item>;
  /** Items in slots the grid has no place for (e.g. added by mods); listed under it so nothing is hidden. */
  unplaced: Item[];
}

/** The items of a container keyed by the slot of the grid they go in. */
export const slotItems = (items: readonly Item[], container: 'inventory' | 'ender_chest'): SlotItems => {
  const layout = container === 'inventory' ? INVENTORY_LAYOUT : ENDER_CHEST_LAYOUT;
  const bySlot = new Map<string, Item>();
  const unplaced: Item[] = [];
  for (const item of items) {
    if (layout.has(item.slot) && !bySlot.has(item.slot)) bySlot.set(item.slot, item);
    else unplaced.push(item);
  }
  return { bySlot, unplaced };
};

/** Every item id of the profile, for the autocomplete of the give dialog. */
export const seenItemIds = (profile: Pick<Profile, 'inventory' | 'ender_chest'>): string[] =>
  [...new Set([...profile.inventory, ...profile.ender_chest].map((item) => item.id))].sort();

// ---- formatting

const ROMAN: [number, string][] = [
  [1000, 'M'],
  [900, 'CM'],
  [500, 'D'],
  [400, 'CD'],
  [100, 'C'],
  [90, 'XC'],
  [50, 'L'],
  [40, 'XL'],
  [10, 'X'],
  [9, 'IX'],
  [5, 'V'],
  [4, 'IV'],
  [1, 'I'],
];

/** Enchantment and effect levels the way the game shows them; levels without a numeral stay digits. */
export const romanNumeral = (value: number): string => {
  if (!Number.isInteger(value) || value < 1 || value > 3999) return String(value);

  let rest = value;
  let numeral = '';
  for (const [amount, symbol] of ROMAN) {
    while (rest >= amount) {
      numeral += symbol;
      rest -= amount;
    }
  }
  return numeral;
};

export const TICKS_PER_SECOND = 20;

/** An effect duration as a clock (`m:ss`, `h:mm:ss`); null for infinite effects (negative durations). */
export const formatTicksClock = (ticks: number): string | null => {
  if (ticks < 0) return null;

  const total = Math.floor(ticks / TICKS_PER_SECOND);
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  const seconds = String(total % 60).padStart(2, '0');
  return hours > 0 ? `${hours}:${String(minutes).padStart(2, '0')}:${seconds}` : `${minutes}:${seconds}`;
};

const DURATION_UNITS = [
  ['days', 86_400],
  ['hours', 3600],
  ['minutes', 60],
  ['seconds', 1],
] as const;

/** A long duration in ticks (play time) by its two largest units, e.g. `3d 4h`, `12m 5s`. */
export const formatTicksDuration = (ticks: number, locale?: string): string => {
  let rest = Math.max(0, Math.floor(ticks / TICKS_PER_SECOND));
  const parts: Partial<Record<(typeof DURATION_UNITS)[number][0], number>> = {};
  let used = 0;
  for (const [unit, seconds] of DURATION_UNITS) {
    const amount = Math.floor(rest / seconds);
    rest -= amount * seconds;
    if (used > 0 || amount > 0) {
      if (amount > 0) parts[unit] = amount;
      used += 1;
    }
    if (used === 2) break;
  }
  // a zero duration still reads as `0s`
  if (used === 0)
    return new Intl.DurationFormat(locale, { style: 'narrow', secondsDisplay: 'always' }).format({ seconds: 0 });
  return new Intl.DurationFormat(locale, { style: 'narrow' }).format(parts);
};

/** A distance in centimeters (the unit of the game's movement stats) in km from 1 km on, else in whole meters. */
export const formatCentimeters = (cm: number, locale?: string): string => {
  const meters = cm / 100;
  // 999.5 m and up would round to "1,000 m"
  if (meters >= 999.5) {
    return new Intl.NumberFormat(locale, { style: 'unit', unit: 'kilometer', maximumFractionDigits: 1 }).format(
      meters / 1000,
    );
  }
  return new Intl.NumberFormat(locale, { style: 'unit', unit: 'meter', maximumFractionDigits: 0 }).format(meters);
};

/** How a stat value is counted. */
export type StatFormat = 'count' | 'ticks' | 'centimeters' | 'tenthHealth';

/** A stat value with its unit; health is counted in tenths of a health point, shown as hearts (2 points). */
export const formatStat = (format: StatFormat, value: number, locale?: string): string => {
  switch (format) {
    case 'ticks':
      return formatTicksDuration(value, locale);
    case 'centimeters':
      return formatCentimeters(value, locale);
    case 'tenthHealth':
      return `${new Intl.NumberFormat(locale, { maximumFractionDigits: 1 }).format(value / 20)} ❤`;
    case 'count':
      return new Intl.NumberFormat(locale, { maximumFractionDigits: 0 }).format(value);
  }
};

/** A readable name of a namespaced id: the words of its last path segment (`minecraft:story/mine_stone` → Mine Stone). */
export const prettifyId = (id: string): string => {
  const path = id.slice(id.indexOf(':') + 1);
  const name = path
    .slice(path.lastIndexOf('/') + 1)
    .split(/[_\s]+/)
    .filter(Boolean)
    .map((word) => word[0].toUpperCase() + word.slice(1))
    .join(' ');
  return name || id;
};

/** The namespace of an id, `minecraft` when it has none. */
export const idNamespace = (id: string): string => (id.includes(':') ? id.slice(0, id.indexOf(':')) : 'minecraft');

/** Up to two letters standing in for an item without an icon: the initials of the words of its path. */
export const idInitials = (id: string): string => {
  const path = id.slice(id.indexOf(':') + 1);
  const parts = path
    .slice(path.lastIndexOf('/') + 1)
    .split(/[_.\-\s]+/)
    .filter(Boolean);
  if (parts.length === 0) return '?';
  const letters = parts.length === 1 ? parts[0].slice(0, 2) : parts[0][0] + parts[1][0];
  return letters.toUpperCase();
};

/**
 * A timestamp as the game stores it (`2024-05-01 12:34:56 +0200` in advancement files) or any format `Date`
 * understands; null when it is neither.
 */
export const parseStoredDate = (value: string): Date | null => {
  const stored = /^(\d{4}-\d{2}-\d{2}) (\d{2}:\d{2}:\d{2}) ([+-]\d{2})(\d{2})$/.exec(value);
  const date = new Date(stored ? `${stored[1]}T${stored[2]}${stored[3]}:${stored[4]}` : value);
  return Number.isNaN(date.getTime()) ? null : date;
};

/**
 * The fill of each icon of a bar of `max` points shown as icons of `perIcon` points (hearts and hunger shanks
 * hold two): 1 full, 0.5 half, 0 empty. Values round up to half an icon like the game's HUD.
 */
export const iconFills = (value: number, max: number, perIcon = 2): number[] => {
  const icons = Math.max(0, Math.ceil(max / perIcon));
  const halves = Math.min(icons * 2, Math.max(0, Math.ceil((value / perIcon) * 2)));
  return Array.from({ length: icons }, (_, index) => Math.min(1, Math.max(0, (halves - index * 2) / 2)));
};

// ---- SNBT

/** Whether the bracketed text holds another compound or list outside quoted strings. */
const hasNested = (text: string): boolean => {
  let quote: string | null = null;
  for (let index = 0; index < text.length; index++) {
    const char = text[index];
    if (quote) {
      if (char === '\\') index++;
      else if (char === quote) quote = null;
    } else if (char === '"' || char === "'") {
      quote = char;
    } else if (char === '{' || char === '[') {
      return true;
    }
  }
  return false;
};

/** Index of the bracket closing the one at `start`, skipping quoted strings. */
const closingIndex = (snbt: string, start: number): number => {
  let depth = 0;
  let quote: string | null = null;
  for (let index = start; index < snbt.length; index++) {
    const char = snbt[index];
    if (quote) {
      if (char === '\\') index++;
      else if (char === quote) quote = null;
    } else if (char === '"' || char === "'") {
      quote = char;
    } else if (char === '{' || char === '[') {
      depth++;
    } else if (char === '}' || char === ']') {
      depth--;
      if (depth === 0) return index;
    }
  }
  return snbt.length - 1;
};

const INLINE_MAX = 60;

/**
 * Indents SNBT for reading: one entry per line, with short compounds and lists of plain values (enchantment
 * levels, typed arrays) kept on one line. Quoted strings stay untouched.
 */
export const prettySnbt = (snbt: string): string => {
  let out = '';
  let depth = 0;
  let inline = 0;
  let inlineEnd = -1;
  let quote: string | null = null;
  const newline = () => `\n${'  '.repeat(depth)}`;

  for (let index = 0; index < snbt.length; index++) {
    const char = snbt[index];
    if (index === inlineEnd) inline = 0;
    if (quote) {
      out += char;
      if (char === '\\' && index + 1 < snbt.length) out += snbt[++index];
      else if (char === quote) quote = null;
      continue;
    }

    if (char === '"' || char === "'") {
      quote = char;
      out += char;
    } else if (/\s/.test(char)) {
      // whitespace outside strings carries no meaning; the layout below replaces it
    } else if (inline > 0) {
      out += char === ',' ? ', ' : char === ':' ? ': ' : char;
    } else if (char === '{' || char === '[') {
      const end = closingIndex(snbt, index);
      const body = snbt.slice(index + 1, end);
      if (body.trim() === '' || (body.length <= INLINE_MAX && !hasNested(body))) {
        inline = 1;
        inlineEnd = end + 1;
        out += char;
      } else {
        depth++;
        out += char + newline();
      }
    } else if (char === '}' || char === ']') {
      depth = Math.max(0, depth - 1);
      out += newline() + char;
    } else if (char === ',') {
      out += `,${newline()}`;
    } else if (char === ':') {
      out += ': ';
    } else {
      out += char;
    }
  }
  return out;
};

// ---- actions

/**
 * How profile actions run right now, as the backend decides it: file edits while the server is offline or the
 * player is not on a complete online list, commands while they are online; `unknown` until a complete online
 * list says otherwise (the backend then checks itself and may refuse), `transition` while starting or stopping.
 */
export type ProfileMode = 'live' | 'file' | 'unknown' | 'transition';

export const profileMode = (state: ServerState, online: OnlinePlayers | undefined, player: PlayerRef): ProfileMode => {
  if (state === 'offline') return 'file';
  if (state !== 'running') return 'transition';
  if (!online) return 'unknown';

  const id = player.id?.replaceAll('-', '').toLowerCase() ?? null;
  const listed = online.players.some(
    (entry) =>
      (id !== null && entry.id !== null && entry.id.replaceAll('-', '').toLowerCase() === id) ||
      sameName(entry.name, player.name),
  );
  if (listed) return 'live';
  return online.complete ? 'file' : 'unknown';
};

/**
 * Access to a profile action in the current mode. `give` only exists as a command, so it needs the player
 * online; everything else runs with whichever of the two capabilities the mode picks, and while the mode is
 * unknown with either (the backend decides).
 */
export const profileActionAccess = (mode: ProfileMode, live: Access, offline: Access, give: boolean): Access => {
  const visible = give ? live.visible : live.visible || offline.visible;
  if (mode === 'transition') return { visible, blocker: 'transition' };
  if (give) return mode === 'file' ? { visible, blocker: 'playerOffline' } : live;

  switch (mode) {
    case 'live':
      return { visible, blocker: live.visible ? live.blocker : 'noConsole' };
    case 'file':
      return { visible, blocker: offline.visible ? offline.blocker : 'noFiles' };
    case 'unknown': {
      const usable = (live.visible && live.blocker === null) || (offline.visible && offline.blocker === null);
      return { visible, blocker: usable ? null : (live.blocker ?? offline.blocker ?? 'noPermission') };
    }
  }
};
