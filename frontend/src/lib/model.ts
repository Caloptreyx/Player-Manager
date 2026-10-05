// the shared types of the API contract; every game maps onto them, the UI renders from them alone

export const SERVER_STATES = ['offline', 'starting', 'stopping', 'running'] as const;
export type ServerState = (typeof SERVER_STATES)[number];

/** The player lists games map onto, in the order the page shows them. */
export const LIST_KINDS = ['whitelist', 'operators', 'bans', 'ip_bans'] as const;
export type ListKind = (typeof LIST_KINDS)[number];

export const METHODS = ['command', 'file'] as const;
export type Method = (typeof METHODS)[number];

export interface Player {
  name: string;
  id: string | null;
}

/** One row of any list of any game; fields a list does not use are null. */
export interface Entry {
  /** Null e.g. for Bedrock permissions.json rows no known player matches. */
  name: string | null;
  /** Game player id (Java UUID, Bedrock XUID). */
  id: string | null;
  /** IP bans only. */
  ip: string | null;
  /** Operators: one of `ListSpec.levels.options`. */
  level: string | null;
  bypasses_player_limit: boolean | null;
  reason: string | null;
  source: string | null;
  created: string | null;
  expires: string | null;
}

export const BLOCKED = ['transition', 'not_running'] as const;

/** An action of the game, with the panel permissions it needs in the current server state. */
export interface Capability {
  /** All of these are needed right now. */
  requires: string[];
  /** The control is hidden when the user holds none of these: no server state would allow it. */
  visible_with: string[];
  /** Why the current server state forbids the action. */
  blocked: (typeof BLOCKED)[number] | null;
}

export interface MethodCapability extends Capability {
  method: Method;
}

export interface KickCapability extends Capability {
  /** Whether a kick accepts a reason. */
  reason: boolean;
}

export interface ListSpec {
  kind: ListKind;
  target: 'player' | 'ip';
  /** Whether adds accept a player id in the current mode. */
  id: 'none' | 'optional';
  /** Whether adds accept a reason. */
  reason: boolean;
  /** Null: no level choice in the current mode. */
  levels: { options: string[]; default: string } | null;
  /** Whether adds accept the player limit flag in the current mode. */
  bypasses_player_limit: boolean;
}

export interface Game {
  id: string;
  family: string;
  /** Anchored regex source the backend validates names with too. */
  player_name: { pattern: string };
  player_id: { kind: string; pattern: string };
  /** In display order. */
  lists: ListSpec[];
  /** List adds and removes. */
  edit: MethodCapability | null;
  whitelist_toggle: MethodCapability | null;
  online: Capability | null;
  kick: KickCapability | null;
}

export interface FileError {
  file: string;
  message: string;
}

export interface Overview {
  /** Null: no supported game detected; everything else is empty. */
  game: Game | null;
  state: ServerState;
  info: { whitelist_enabled: boolean | null; max_players: number | null; online_mode: boolean | null };
  /** Exactly the kinds in `game.lists`. */
  lists: Partial<Record<ListKind, Entry[]>>;
  known: Player[];
  errors: FileError[];
}

export interface OnlinePlayers {
  count: number;
  max: number;
  players: Player[];
}

export interface MutationResult {
  method: Method;
  restart_required: boolean;
}
