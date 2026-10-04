import { z } from 'zod';
import { axiosInstance } from '@/api/axios.ts';
import {
  type Ban,
  BEDROCK_LEVELS,
  EDITIONS,
  type IpBan,
  type OnlinePlayers,
  type Operator,
  type OperatorLevel,
  type Overview,
  type Player,
  SERVER_STATES,
  type WhitelistEntry,
} from './lib/players.ts';

export const playerManagerBase = (serverUuid: string) => `/api/client/servers/${serverUuid}/player-manager`;

export const playerManagerQueryKey = (serverUuid: string) => ['dev.caloptreyx.playermanager', serverUuid] as const;

// the panel's axios instance does not transform keys, so payloads go out and come back in the contract's
// snake_case exactly as written here

const playerSchema: z.ZodType<Player> = z.object({ name: z.string(), id: z.string().nullable() });

const whitelistEntrySchema: z.ZodType<WhitelistEntry> = z.object({
  name: z.string(),
  id: z.string().nullable(),
  ignores_player_limit: z.boolean().nullable(),
});

const operatorSchema: z.ZodType<Operator> = z.object({
  name: z.string().nullable(),
  id: z.string().nullable(),
  level: z.union([z.number(), z.enum(BEDROCK_LEVELS)]),
  bypasses_player_limit: z.boolean().nullable(),
});

const banSchema: z.ZodType<Ban> = z.object({
  name: z.string(),
  id: z.string().nullable(),
  reason: z.string().nullable(),
  source: z.string().nullable(),
  created: z.string().nullable(),
  expires: z.string().nullable(),
});

const ipBanSchema: z.ZodType<IpBan> = z.object({
  ip: z.string(),
  reason: z.string().nullable(),
  source: z.string().nullable(),
  created: z.string().nullable(),
  expires: z.string().nullable(),
});

const overviewSchema: z.ZodType<Overview> = z.object({
  edition: z.enum(EDITIONS).nullable(),
  state: z.enum(SERVER_STATES),
  online_mode: z.boolean().nullable(),
  whitelist_enabled: z.boolean().nullable(),
  max_players: z.number().nullable(),
  whitelist: z.array(whitelistEntrySchema),
  operators: z.array(operatorSchema),
  bans: z.array(banSchema),
  ip_bans: z.array(ipBanSchema),
  known: z.array(playerSchema),
  errors: z.array(z.object({ file: z.string(), message: z.string() })),
});

const onlinePlayersSchema: z.ZodType<OnlinePlayers> = z.object({
  count: z.number(),
  max: z.number(),
  players: z.array(playerSchema),
});

const mutationResultSchema = z.object({
  method: z.enum(['command', 'file']),
  restart_required: z.boolean(),
});
export type MutationResult = z.infer<typeof mutationResultSchema>;

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

export const addToWhitelist = (
  serverUuid: string,
  body: { name: string; id?: string; ignores_player_limit?: boolean },
) => mutate('post', serverUuid, '/whitelist', body);

export const removeFromWhitelist = (serverUuid: string, name: string) =>
  mutate('delete', serverUuid, '/whitelist', { name });

export const setWhitelistEnabled = (serverUuid: string, enabled: boolean) =>
  mutate('put', serverUuid, '/whitelist/enabled', { enabled });

export const addOperator = (
  serverUuid: string,
  body: { name: string; id?: string; level?: OperatorLevel; bypasses_player_limit?: boolean },
) => mutate('post', serverUuid, '/operators', body);

export const removeOperator = (serverUuid: string, body: { name?: string; id?: string }) =>
  mutate('delete', serverUuid, '/operators', body);

export const addBan = (serverUuid: string, body: { name: string; id?: string; reason?: string }) =>
  mutate('post', serverUuid, '/bans', body);

export const removeBan = (serverUuid: string, name: string) => mutate('delete', serverUuid, '/bans', { name });

export const addIpBan = (serverUuid: string, body: { ip: string; reason?: string }) =>
  mutate('post', serverUuid, '/ip-bans', body);

export const removeIpBan = (serverUuid: string, ip: string) => mutate('delete', serverUuid, '/ip-bans', { ip });

export const kickPlayer = (serverUuid: string, body: { name: string; reason?: string }) =>
  mutate('post', serverUuid, '/kick', body);
