import { z } from 'zod';
import { axiosInstance } from '@/api/axios.ts';
import type { AddBody, RemoveBody } from './lib/lists.ts';
import {
  BLOCKED,
  type Capability,
  type Entry,
  type Game,
  type KickCapability,
  LIST_KINDS,
  type ListKind,
  type ListSpec,
  METHODS,
  type MethodCapability,
  type MutationResult,
  type OnlinePlayers,
  type Overview,
  type Player,
  SERVER_STATES,
} from './lib/model.ts';

export const playerManagerBase = (serverUuid: string) => `/api/client/servers/${serverUuid}/player-manager`;

export const playerManagerQueryKey = (serverUuid: string) => ['dev.caloptreyx.playermanager', serverUuid] as const;

// the panel's axios instance does not transform keys, so payloads go out and come back in the contract's
// snake_case exactly as written here

const playerSchema: z.ZodType<Player> = z.object({ name: z.string(), id: z.string().nullable() });

const entrySchema: z.ZodType<Entry> = z.object({
  name: z.string().nullable(),
  id: z.string().nullable(),
  ip: z.string().nullable(),
  level: z.string().nullable(),
  bypasses_player_limit: z.boolean().nullable(),
  reason: z.string().nullable(),
  source: z.string().nullable(),
  created: z.string().nullable(),
  expires: z.string().nullable(),
});

const capabilityShape = {
  requires: z.array(z.string()),
  visible_with: z.array(z.string()),
  blocked: z.enum(BLOCKED).nullable(),
};

const capabilitySchema: z.ZodType<Capability> = z.object(capabilityShape);

const methodCapabilitySchema: z.ZodType<MethodCapability> = z.object({
  ...capabilityShape,
  method: z.enum(METHODS),
});

const kickCapabilitySchema: z.ZodType<KickCapability> = z.object({ ...capabilityShape, reason: z.boolean() });

const listSpecSchema: z.ZodType<ListSpec> = z.object({
  kind: z.enum(LIST_KINDS),
  target: z.enum(['player', 'ip']),
  id: z.enum(['none', 'optional']),
  reason: z.boolean(),
  levels: z.object({ options: z.array(z.string()), default: z.string() }).nullable(),
  bypasses_player_limit: z.boolean(),
});

const gameSchema: z.ZodType<Game> = z.object({
  id: z.string(),
  family: z.string(),
  player_name: z.object({ pattern: z.string() }),
  player_id: z.object({ kind: z.string(), pattern: z.string() }),
  lists: z.array(listSpecSchema),
  edit: methodCapabilitySchema.nullable(),
  whitelist_toggle: methodCapabilitySchema.nullable(),
  online: capabilitySchema.nullable(),
  kick: kickCapabilitySchema.nullable(),
});

const overviewSchema: z.ZodType<Overview> = z.object({
  game: gameSchema.nullable(),
  state: z.enum(SERVER_STATES),
  info: z.object({
    whitelist_enabled: z.boolean().nullable(),
    max_players: z.number().nullable(),
    online_mode: z.boolean().nullable(),
  }),
  lists: z.partialRecord(z.enum(LIST_KINDS), z.array(entrySchema)),
  known: z.array(playerSchema),
  errors: z.array(z.object({ file: z.string(), message: z.string() })),
});

const onlinePlayersSchema: z.ZodType<OnlinePlayers> = z.object({
  count: z.number(),
  max: z.number(),
  players: z.array(playerSchema),
});

const mutationResultSchema: z.ZodType<MutationResult> = z.object({
  method: z.enum(METHODS),
  restart_required: z.boolean(),
});

export const getOverview = async (serverUuid: string): Promise<Overview> => {
  const { data } = await axiosInstance.get(playerManagerBase(serverUuid));
  return overviewSchema.parse(data);
};

export const getOnlinePlayers = async (serverUuid: string): Promise<OnlinePlayers> => {
  const { data } = await axiosInstance.get(`${playerManagerBase(serverUuid)}/online`);
  return onlinePlayersSchema.parse(data);
};

const mutate = async (
  method: 'post' | 'put' | 'delete',
  serverUuid: string,
  path: string,
  body: object,
): Promise<MutationResult> => {
  const url = `${playerManagerBase(serverUuid)}${path}`;
  const { data } =
    method === 'delete' ? await axiosInstance.delete(url, { data: body }) : await axiosInstance[method](url, body);
  return mutationResultSchema.parse(data);
};

export const addToList = (serverUuid: string, kind: ListKind, body: AddBody) =>
  mutate('post', serverUuid, `/lists/${kind}`, body);

export const removeFromList = (serverUuid: string, kind: ListKind, body: RemoveBody) =>
  mutate('delete', serverUuid, `/lists/${kind}`, body);

export const setWhitelistEnabled = (serverUuid: string, enabled: boolean) =>
  mutate('put', serverUuid, '/whitelist', { enabled });

export const kickPlayer = (serverUuid: string, body: { name: string; reason?: string }) =>
  mutate('post', serverUuid, '/kick', body);
