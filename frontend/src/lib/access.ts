import type { Edition, ServerState } from './players.ts';

/** The panel permissions the routes of this extension check. */
export interface Granted {
  /** `control.console`: send console commands. */
  console: boolean;
  /** `control.read-console`: read the console output. */
  readConsole: boolean;
  /** `files.create`: what the panel's own file write route requires. */
  writeFiles: boolean;
}

export type Blocker = 'transition' | 'notRunning' | 'noConsole' | 'noFiles';

export interface Access {
  /** False when the user could not do this in any server state; the control is hidden. */
  visible: boolean;
  /** Why the control is disabled right now, null when it is usable. */
  blocker: Blocker | null;
}

export type Method = 'command' | 'file';

const isTransition = (state: ServerState) => state === 'starting' || state === 'stopping';

/** How list mutations are carried out right now, null while the server is starting or stopping. */
export const mutationMethod = (edition: Edition, state: ServerState): Method | null => {
  if (isTransition(state)) return null;
  return edition === 'java' && state === 'running' ? 'command' : 'file';
};

/** Adding to or removing from the whitelist, operators, bans and ip bans. */
export const listAccess = (edition: Edition, state: ServerState, granted: Granted): Access => {
  const visible = edition === 'java' ? granted.console || granted.writeFiles : granted.writeFiles;
  if (isTransition(state)) return { visible, blocker: 'transition' };

  const running = state === 'running';
  // Java running: console commands only; Bedrock running: file edit plus a reload command
  const needsConsole = running;
  const needsFiles = edition === 'bedrock' || !running;
  if (needsFiles && !granted.writeFiles) return { visible, blocker: 'noFiles' };
  if (needsConsole && !granted.console) return { visible, blocker: 'noConsole' };
  return { visible, blocker: null };
};

/** `PUT /whitelist/enabled`: a command on a running Java server, a server.properties edit otherwise. */
export const whitelistToggleAccess = (edition: Edition, state: ServerState, granted: Granted): Access => {
  const visible = edition === 'java' ? granted.console || granted.writeFiles : granted.writeFiles;
  if (isTransition(state)) return { visible, blocker: 'transition' };
  if (edition === 'java' && state === 'running') return { visible, blocker: granted.console ? null : 'noConsole' };
  return { visible, blocker: granted.writeFiles ? null : 'noFiles' };
};

export const kickAccess = (state: ServerState, granted: Granted): Access => ({
  visible: granted.console,
  blocker: state === 'running' ? null : 'notRunning',
});

/** `GET /online` sends `list` and reads its answer from the console. */
export const onlineAccess = (state: ServerState, granted: Granted): Access => ({
  visible: granted.console && granted.readConsole,
  blocker: state === 'running' ? null : isTransition(state) ? 'transition' : 'notRunning',
});
