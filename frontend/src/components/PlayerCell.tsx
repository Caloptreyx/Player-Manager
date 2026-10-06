import type { IconDefinition } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import classNames from 'classnames';
import CopyOnClick from '@/elements/CopyOnClick.tsx';
import Avatar from '@/elements/data-display/Avatar.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import type { PlayerRef } from '../lib/players.ts';
import { usePlayerManager, useText } from './playerManager.ts';

// avatar, name and click-to-copy id of a player; rows without a player (ip bans) pass an `icon` and `title`,
// which is copyable too
export default function PlayerCell({
  player,
  icon,
  title,
  size = 32,
}: {
  player?: PlayerRef;
  icon?: IconDefinition;
  title?: string;
  /** Avatar size in pixels. */
  size?: 32 | 48;
}) {
  const text = useText();
  const { ui } = usePlayerManager();

  const name = title ?? player?.name ?? null;
  const copyable = title ?? player?.id ?? null;
  const large = size === 48;

  return (
    <div className='flex min-w-0 items-center gap-3'>
      {icon ? (
        <div
          className='grid shrink-0 place-items-center rounded-md bg-(--mantine-color-default-hover) text-(--mantine-color-dimmed)'
          style={{ width: size, height: size }}
        >
          <FontAwesomeIcon icon={icon} />
        </div>
      ) : (
        <Avatar
          size={size}
          radius='md'
          className='shrink-0'
          src={player ? ui.avatarUrl(player, size * 2) : null}
          name={name ?? '?'}
        />
      )}

      <div className='flex min-w-0 flex-col items-start'>
        {title === undefined && (
          <span
            className={classNames(
              'max-w-full truncate',
              large ? 'text-base' : 'text-sm',
              name === null ? 'italic text-(--mantine-color-dimmed)' : 'font-medium',
            )}
          >
            {name ?? text('common.unknownPlayer', {})}
          </span>
        )}
        {copyable && (
          <Tooltip label={text('common.copyId', {})} className='max-w-full' innerClassName='max-w-full'>
            <CopyOnClick
              content={copyable}
              className={classNames(
                'block max-w-full truncate text-left hover:underline',
                title === undefined
                  ? 'font-mono text-xs text-(--mantine-color-dimmed)'
                  : 'font-mono text-sm font-medium',
              )}
            >
              {copyable}
            </CopyOnClick>
          </Tooltip>
        )}
      </div>
    </div>
  );
}
