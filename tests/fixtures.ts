import type { Entry, Game, ListSpec } from '../frontend/src/lib/model.ts';

// game descriptors as the backend sends them for Minecraft (contract v2)

const CONSOLE = 'control.console';
const READ_CONSOLE = 'control.read-console';
const FILES = 'files.create';

export const entry = (fields: Partial<Entry>): Entry => ({
  name: null,
  id: null,
  ip: null,
  level: null,
  bypasses_player_limit: null,
  reason: null,
  source: null,
  created: null,
  expires: null,
  ...fields,
});

const javaLists = (fileMode: boolean): ListSpec[] => {
  const id = fileMode ? 'optional' : 'none';
  return [
    { kind: 'whitelist', target: 'player', id, reason: false, levels: null, bypasses_player_limit: false },
    {
      kind: 'operators',
      target: 'player',
      id,
      reason: false,
      levels: fileMode ? { options: ['1', '2', '3', '4'], default: '4' } : null,
      bypasses_player_limit: fileMode,
    },
    { kind: 'bans', target: 'player', id, reason: true, levels: null, bypasses_player_limit: false },
    { kind: 'ip_bans', target: 'ip', id: 'none', reason: true, levels: null, bypasses_player_limit: false },
  ];
};

export const javaGame = (state: 'running' | 'offline' | 'starting'): Game => {
  const running = state === 'running';
  const edit = {
    requires: running ? [CONSOLE] : [FILES],
    visible_with: [CONSOLE, FILES],
    blocked: state === 'starting' ? ('transition' as const) : null,
    method: running ? ('command' as const) : ('file' as const),
  };
  return {
    id: 'minecraft_java',
    family: 'minecraft',
    player_name: { pattern: '^[.*]?[A-Za-z0-9_]{1,16}$' },
    player_id: {
      kind: 'uuid',
      pattern:
        '^(?:[0-9a-fA-F]{32}|[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12})$',
    },
    lists: javaLists(!running),
    edit,
    whitelist_toggle: edit,
    online: {
      requires: [CONSOLE, READ_CONSOLE],
      visible_with: [CONSOLE, READ_CONSOLE],
      blocked: running ? null : state === 'starting' ? 'transition' : 'not_running',
    },
    kick: { requires: [CONSOLE], visible_with: [CONSOLE], blocked: running ? null : 'not_running', reason: true },
  };
};

export const bedrockGame = (state: 'running' | 'offline'): Game => {
  const running = state === 'running';
  return {
    id: 'minecraft_bedrock',
    family: 'minecraft',
    player_name: { pattern: '^[A-Za-z0-9](?:[A-Za-z0-9 ]{0,30}[A-Za-z0-9])?$' },
    player_id: { kind: 'xuid', pattern: '^[0-9]{1,20}$' },
    lists: [
      { kind: 'whitelist', target: 'player', id: 'optional', reason: false, levels: null, bypasses_player_limit: true },
      {
        kind: 'operators',
        target: 'player',
        id: 'optional',
        reason: false,
        levels: { options: ['operator', 'member', 'visitor'], default: 'operator' },
        bypasses_player_limit: false,
      },
    ],
    edit: {
      requires: running ? [FILES, CONSOLE] : [FILES],
      visible_with: [FILES],
      blocked: null,
      method: 'file',
    },
    whitelist_toggle: { requires: [FILES], visible_with: [FILES], blocked: null, method: 'file' },
    online: {
      requires: [CONSOLE, READ_CONSOLE],
      visible_with: [CONSOLE, READ_CONSOLE],
      blocked: running ? null : 'not_running',
    },
    kick: { requires: [CONSOLE], visible_with: [CONSOLE], blocked: running ? null : 'not_running', reason: true },
  };
};

export const spec = (game: Game, kind: ListSpec['kind']): ListSpec => {
  const found = game.lists.find((list) => list.kind === kind);
  if (!found) throw new Error(`${game.id} has no ${kind} list`);
  return found;
};
