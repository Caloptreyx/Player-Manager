import {
  faBan,
  faDoorOpen,
  faListCheck,
  faUserMinus,
  faUserShield,
  faUserSlash,
  type IconDefinition,
} from '@fortawesome/free-solid-svg-icons';
import { addToList, kickPlayer, removeFromList } from '../api.ts';
import type { Access } from '../lib/access.ts';
import { addBody, removeBody } from '../lib/lists.ts';
import type { ListKind } from '../lib/model.ts';
import { type PlayerRef, playerStatus } from '../lib/players.ts';
import { usePlayerManager, useText } from './playerManager.ts';

export interface PlayerAction {
  key: string;
  icon: IconDefinition;
  label: string;
  access: Access;
  /** Keeps the player out or disconnects them. */
  danger?: boolean;
  onClick: () => void;
}

/**
 * What can be done with a player besides the row's own list: whitelist and operator toggles, ban and kick, as
 * far as the game supports them. `except` leaves out the list a row belongs to; `onGone` runs after a ban or
 * kick disconnected the player; `kick` only makes sense for online players.
 */
export function usePlayerActions(
  player: PlayerRef,
  { except, kick: withKick = false, onGone }: { except?: ListKind; kick?: boolean; onGone?: (name: string) => void },
): PlayerAction[] {
  const text = useText();
  const { serverUuid, game, ui, overview, editAccess, kickAccess, run, confirm } = usePlayerManager();
  const status = playerStatus(overview.lists, player);
  const { id } = player;
  const label = player.name ?? id ?? '';
  const spec = (kind: ListKind) => (kind === except ? undefined : game.lists.find((list) => list.kind === kind));
  const whitelist = spec('whitelist');
  const operators = spec('operators');
  const bans = spec('bans');
  const { whitelist: listed, operators: operator, bans: banned } = status;
  // Bedrock members and visitors are in permissions.json too, but only operators count as opped
  const opped = operator !== undefined && (operator.level === null || ui.isOperatorLevel(operator.level));
  // adds go by name; a Bedrock operator known only by xuid cannot be added elsewhere
  const name = player.name;
  const actions: PlayerAction[] = [];

  if (whitelist && listed) {
    actions.push({
      key: 'whitelist',
      icon: faUserMinus,
      label: text('lists.whitelist.remove', {}),
      access: editAccess,
      onClick: () =>
        confirm({
          title: text('lists.removeTitle', { name: label }),
          content: text('lists.whitelist.removeContent', { name: label }),
          confirm: text('lists.whitelist.removeConfirm', {}),
          run: () => removeFromList(serverUuid, 'whitelist', removeBody(whitelist, listed)),
          success: text('lists.whitelist.removed', { name: label }),
        }),
    });
  } else if (whitelist && name !== null) {
    actions.push({
      key: 'whitelist',
      icon: faListCheck,
      label: text('lists.whitelist.addOnline', {}),
      access: editAccess,
      onClick: () =>
        run(
          () => addToList(serverUuid, 'whitelist', addBody(whitelist, { name, id })),
          text('lists.whitelist.added', { name }),
        ),
    });
  }

  if (operators && opped) {
    actions.push({
      key: 'operators',
      icon: faUserSlash,
      label: text('lists.operators.remove', {}),
      access: editAccess,
      onClick: () =>
        confirm({
          title: text('lists.removeTitle', { name: label }),
          content: text('lists.operators.removeContent', { name: label }),
          confirm: text('lists.operators.removeConfirm', {}),
          run: () => removeFromList(serverUuid, 'operators', removeBody(operators, operator)),
          success: text('lists.operators.removed', { name: label }),
        }),
    });
  } else if (operators && name !== null) {
    actions.push({
      key: 'operators',
      icon: faUserShield,
      label: text('lists.operators.addOnline', {}),
      access: editAccess,
      onClick: () =>
        run(
          () => addToList(serverUuid, 'operators', addBody(operators, { name, id })),
          text('lists.operators.added', { name }),
        ),
    });
  }

  if (bans && !banned && name !== null) {
    actions.push({
      key: 'ban',
      icon: faBan,
      label: text('lists.bans.addOnline', {}),
      access: editAccess,
      danger: true,
      onClick: () =>
        confirm({
          title: text('online.banTitle', { name }),
          content: text('online.banContent', { name }),
          confirm: text('lists.bans.addOnline', {}),
          withReason: bans.reason,
          run: (reason) => addToList(serverUuid, 'bans', addBody(bans, { name, id, reason })),
          success: text('lists.bans.added', { name }),
          onSuccess: () => onGone?.(name),
        }),
    });
  }

  const { kick } = game;
  if (withKick && kick && name !== null) {
    actions.push({
      key: 'kick',
      icon: faDoorOpen,
      label: text('online.kick', {}),
      access: kickAccess,
      danger: true,
      onClick: () =>
        confirm({
          title: text('online.kickTitle', { name }),
          content: text('online.kickContent', { name }),
          confirm: text('online.kick', {}),
          withReason: kick.reason,
          run: (reason) => kickPlayer(serverUuid, { name, reason: reason || undefined }),
          success: text('online.kicked', { name }),
          onSuccess: () => onGone?.(name),
        }),
    });
  }

  return actions;
}
