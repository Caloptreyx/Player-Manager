export const EDITIONS = ['java', 'bedrock'] as const;
export type Edition = (typeof EDITIONS)[number];

export const SERVER_STATES = ['offline', 'starting', 'stopping', 'running'] as const;
export type ServerState = (typeof SERVER_STATES)[number];

export const BEDROCK_LEVELS = ['operator', 'member', 'visitor'] as const;
export type BedrockLevel = (typeof BEDROCK_LEVELS)[number];
export type OperatorLevel = number | BedrockLevel;

export interface Player {
  name: string;
  id: string | null;
}

export interface WhitelistEntry {
  name: string;
  id: string | null;
  ignores_player_limit: boolean | null;
}

export interface Operator {
  name: string | null;
  id: string | null;
  level: OperatorLevel;
  bypasses_player_limit: boolean | null;
}

export interface Ban {
  name: string;
  id: string | null;
  reason: string | null;
  source: string | null;
  created: string | null;
  expires: string | null;
}

export interface IpBan {
  ip: string;
  reason: string | null;
  source: string | null;
  created: string | null;
  expires: string | null;
}

export interface FileError {
  file: string;
  message: string;
}

export interface Overview {
  edition: Edition | null;
  state: ServerState;
  online_mode: boolean | null;
  whitelist_enabled: boolean | null;
  max_players: number | null;
  whitelist: WhitelistEntry[];
  operators: Operator[];
  bans: Ban[];
  ip_bans: IpBan[];
  known: Player[];
  errors: FileError[];
}

export interface OnlinePlayers {
  count: number;
  max: number;
  players: Player[];
}

/** Anything that names a player; operators on Bedrock may only carry an xuid. */
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

export interface PlayerStatus {
  operator: Operator | null;
  whitelisted: boolean;
  banned: boolean;
}

/** Cross-references a player with the overview lists, for the badges of every row. */
export const playerStatus = (
  overview: Pick<Overview, 'whitelist' | 'operators' | 'bans'>,
  player: PlayerRef,
): PlayerStatus => ({
  operator: overview.operators.find((operator) => samePlayer(operator, player)) ?? null,
  whitelisted: overview.whitelist.some((entry) => samePlayer(entry, player)),
  banned: overview.bans.some((ban) => samePlayer(ban, player)),
});

/** Java avatars come from mc-heads (uuid preferred, name otherwise); Bedrock has no public skin service. */
export const avatarUrl = (edition: Edition, player: PlayerRef): string | null => {
  if (edition !== 'java') return null;
  const key = player.id ? normalizeId(player.id) : player.name;
  return key ? `https://mc-heads.net/avatar/${encodeURIComponent(key)}/32` : null;
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
export const refetchDelay = (method: 'command' | 'file'): number =>
  method === 'command' ? COMMAND_REFETCH_DELAY_MS : 0;
