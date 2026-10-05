import type { Capability, Game, MethodCapability } from './model.ts';

/** Why a control is disabled; permission blockers name the missing panel permission. */
export type Blocker = 'transition' | 'notRunning' | 'noConsole' | 'noReadConsole' | 'noFiles' | 'noPermission';

export interface Access {
  /** False when the user could not do this in any server state; the control is hidden. */
  visible: boolean;
  /** Why the control is disabled right now, null when it is usable. */
  blocker: Blocker | null;
}

/** A game without the capability: the control does not exist. */
const NO_ACCESS: Access = { visible: false, blocker: null };

// permissions with a dedicated explanation; any other one gets the generic "no permission" text
const PERMISSION_BLOCKERS: Record<string, Blocker> = {
  'control.console': 'noConsole',
  'control.read-console': 'noReadConsole',
  'files.create': 'noFiles',
};

/**
 * Hidden when the user holds none of `visible_with`; otherwise blocked by the server state first, then by the
 * first permission of `requires` the user lacks.
 */
export const capabilityAccess = (capability: Capability | null, granted: ReadonlySet<string>): Access => {
  if (!capability) return NO_ACCESS;

  const visible = capability.visible_with.some((permission) => granted.has(permission));
  if (capability.blocked)
    return { visible, blocker: capability.blocked === 'transition' ? 'transition' : 'notRunning' };

  const missing = capability.requires.find((permission) => !granted.has(permission));
  return { visible, blocker: missing === undefined ? null : (PERMISSION_BLOCKERS[missing] ?? 'noPermission') };
};

/** Every panel permission the capabilities of a game mention, to check them all with one hook call. */
export const gamePermissions = (game: Game | null): string[] => {
  if (!game) return [];

  const permissions = new Set<string>();
  for (const capability of [game.edit, game.whitelist_toggle, game.online, game.kick]) {
    for (const permission of [...(capability?.requires ?? []), ...(capability?.visible_with ?? [])]) {
      permissions.add(permission);
    }
  }
  return [...permissions];
};

export type EditHint = 'command' | 'file' | 'fileReload' | 'transition' | 'notRunning';

/** How list changes are carried out right now, for the hint above the tabs; null when the game has no edits. */
export const editHint = (edit: MethodCapability | null): EditHint | null => {
  if (!edit) return null;
  if (edit.blocked) return edit.blocked === 'transition' ? 'transition' : 'notRunning';
  if (edit.method === 'command') return 'command';
  // a file edit that also needs the console reloads the files with a command (Bedrock while running)
  return edit.requires.includes('control.console') ? 'fileReload' : 'file';
};
