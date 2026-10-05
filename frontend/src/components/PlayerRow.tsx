import type { IconDefinition } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import type { ReactNode } from 'react';
import CopyOnClick from '@/elements/CopyOnClick.tsx';
import Avatar from '@/elements/data-display/Avatar.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import type { PlayerRef } from '../lib/players.ts';
import { usePlayerManager, useText } from './playerManager.ts';

// one line of a player list: avatar, name with badges, copyable id, optional details and the row actions;
// rows without a player (ip bans) pass an `icon` and `title` instead
export default function PlayerRow({
  player,
  icon,
  title,
  badges,
  details,
  actions,
}: {
  player?: PlayerRef;
  icon?: IconDefinition;
  title?: string;
  badges?: ReactNode;
  details?: ReactNode;
  actions?: ReactNode;
}) {
  const text = useText();
  const { ui } = usePlayerManager();

  const name = title ?? player?.name ?? null;
  const id = player?.id ?? null;

  return (
    <div className='flex items-center gap-3 rounded-md px-3 py-2 transition-colors hover:bg-(--mantine-color-default-hover)'>
      {icon ? (
        <div className='grid h-8 w-8 shrink-0 place-items-center rounded-sm bg-(--mantine-color-default-hover) text-(--mantine-color-dimmed)'>
          <FontAwesomeIcon icon={icon} />
        </div>
      ) : (
        <Avatar
          size={32}
          radius='sm'
          className='shrink-0'
          src={player ? ui.avatarUrl(player) : null}
          name={name ?? '?'}
        />
      )}

      <div className='flex min-w-0 flex-1 flex-col gap-0.5'>
        <div className='flex min-w-0 flex-wrap items-center gap-1.5'>
          <span
            className={name === null ? 'text-sm italic text-(--mantine-color-dimmed)' : 'truncate text-sm font-medium'}
          >
            {name ?? text('common.unknownPlayer', {})}
          </span>
          {badges}
        </div>
        {id && (
          <Tooltip label={text('common.copyId', {})}>
            <CopyOnClick content={id} className='truncate text-left font-mono text-xs text-(--mantine-color-dimmed)'>
              {id}
            </CopyOnClick>
          </Tooltip>
        )}
        {details && <div className='text-xs text-(--mantine-color-dimmed)'>{details}</div>}
      </div>

      {actions && <div className='flex shrink-0 items-center gap-0.5'>{actions}</div>}
    </div>
  );
}
