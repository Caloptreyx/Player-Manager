import type { Entry, ListKind, Method, OnlinePlayers, Overview, Player } from './model.ts';

/** Anything that names a player; Bedrock operators may only carry an xuid. */
export interface PlayerRef {
  name: string | null;
  id: string | null;
}

export const sameName = (a: string | null, b: string | null): boolean =>
  a !== null && b !== null && a.toLowerCase() === b.toLowerCase();

// uuids compare without dashes and case, xuids are plain digits so the same normalization is harmless
const normalizeId = (id: string) => id.replaceAll('-', '').toLowerCase();

/** Same player: ids decide when both sides have one, otherwise names compare case-insensitively. */
export const samePlayer = (a: PlayerRef, b: PlayerRef): boolean =>
  a.id !== null && b.id !== null ? normalizeId(a.id) === normalizeId(b.id) : sameName(a.name, b.name);

/** The lists that hold players and get a badge on the rows of the other lists. */
const PLAYER_LIST_KINDS = ['operators', 'whitelist', 'bans'] as const satisfies readonly ListKind[];
type PlayerListKind = (typeof PLAYER_LIST_KINDS)[number];

/** The entry of the player on every player list the game has, for the cross-reference badges of a row. */
export type PlayerStatus = Partial<Record<PlayerListKind, Entry>>;

export const playerStatus = (lists: Overview['lists'], player: PlayerRef): PlayerStatus => {
  const status: PlayerStatus = {};
  for (const kind of PLAYER_LIST_KINDS) {
    const entry = lists[kind]?.find((candidate) => samePlayer(candidate, player));
    if (entry) status[kind] = entry;
  }
  return status;
};

const collator = new Intl.Collator('en', { sensitivity: 'base', numeric: true });

/** Sorts case-insensitively by the given label; entries without a label go last, ordered by id. */
export const sortBy = <T>(items: readonly T[], label: (item: T) => string | null, id?: (item: T) => string | null) =>
  [...items].sort((a, b) => {
    const la = label(a);
    const lb = label(b);
    if (la === null || lb === null) {
      if (la !== lb) return la === null ? 1 : -1;
      return collator.compare((id && id(a)) ?? '', (id && id(b)) ?? '');
    }
    return collator.compare(la, lb);
  });

/** Known player names for the autocomplete of a list, minus the players already on it. */
export const suggestions = (known: readonly Player[], existing: readonly PlayerRef[]): string[] => {
  const names = new Map<string, string>();
  for (const player of known) {
    if (existing.some((entry) => samePlayer(entry, player))) continue;
    const key = player.name.toLowerCase();
    if (!names.has(key)) names.set(key, player.name);
  }
  return [...names.values()].sort(collator.compare);
};

/** Whether any of the values contains the trimmed query, case-insensitively; an empty query matches all. */
export const matchesFilter = (values: readonly (string | null)[], query: string): boolean => {
  const needle = query.trim().toLowerCase();
  return needle === '' || values.some((value) => value?.toLowerCase().includes(needle));
};

/** The id of a known player with this name, used to fill in an id the user left blank. */
export const knownId = (known: readonly Player[], name: string): string | null =>
  known.find((player) => player.id !== null && sameName(player.name, name))?.id ?? null;

/** The online list after a successful kick, without refetching (which would run `list` again). */
export const withoutPlayer = (online: OnlinePlayers, name: string): OnlinePlayers => {
  const players = online.players.filter((player) => !sameName(player.name, name));
  return { ...online, players, count: Math.max(0, online.count - (online.players.length - players.length)) };
};

/** Delay before refetching the overview: console commands make the server write its files asynchronously. */
export const COMMAND_REFETCH_DELAY_MS = 1500;
export const refetchDelay = (method: Method): number => (method === 'command' ? COMMAND_REFETCH_DELAY_MS : 0);
