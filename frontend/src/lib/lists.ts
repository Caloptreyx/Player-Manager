import type { Entry, ListSpec } from './model.ts';
import { type PlayerRef, samePlayer } from './players.ts';

/** `POST /lists/{kind}`; the backend rejects fields the list spec does not accept in the current mode. */
export interface AddBody {
  name?: string;
  id?: string;
  ip?: string;
  level?: string;
  bypasses_player_limit?: boolean;
  reason?: string;
}

/** `DELETE /lists/{kind}`. */
export interface RemoveBody {
  name?: string;
  id?: string;
  ip?: string;
}

/** Whatever the form or row action has; `addBody` keeps what the list accepts. */
export interface AddInput {
  name?: string;
  id?: string | null;
  ip?: string;
  level?: string;
  bypasses_player_limit?: boolean;
  reason?: string;
}

export const addBody = (spec: ListSpec, input: AddInput): AddBody => ({
  ...(spec.target === 'ip' ? { ip: input.ip } : { name: input.name }),
  id: spec.id === 'optional' ? input.id || undefined : undefined,
  level: spec.levels ? (input.level ?? spec.levels.default) : undefined,
  bypasses_player_limit: spec.bypasses_player_limit ? input.bypasses_player_limit : undefined,
  reason: spec.reason ? input.reason || undefined : undefined,
});

/** Identifies an entry by ip, or by name and id (Bedrock operators may only have an id). */
export const removeBody = (spec: ListSpec, entry: Entry): RemoveBody =>
  spec.target === 'ip' ? { ip: entry.ip ?? undefined } : { name: entry.name ?? undefined, id: entry.id ?? undefined };

/**
 * Whether adding would not change the list. Re-adding to a list with levels updates the level, so it is
 * never a duplicate; ips compare case-insensitively (IPv6), players by id or name.
 */
export const isDuplicate = (spec: ListSpec, entries: readonly Entry[], candidate: PlayerRef & { ip?: string }) => {
  if (spec.target === 'ip') {
    const ip = candidate.ip?.toLowerCase();
    return entries.some((entry) => entry.ip?.toLowerCase() === ip);
  }
  return spec.levels === null && entries.some((entry) => samePlayer(entry, candidate));
};
