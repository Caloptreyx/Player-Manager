import { faBan, faPlug, faUsers, type IconDefinition } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Skeleton } from '@mantine/core';
import classNames from 'classnames';
import type { ReactNode } from 'react';
import Card from '@/elements/data-display/Card.tsx';
import ThemeIcon from '@/elements/data-display/ThemeIcon.tsx';
import type { ListKind, OnlinePlayers } from '../lib/model.ts';
import { LIST_STYLES } from './listStyles.ts';
import { usePlayerManager, useText } from './playerManager.ts';

// tailwind needs the full class names in the source
const COLUMNS = ['', 'lg:grid-cols-1', 'lg:grid-cols-2', 'lg:grid-cols-3', 'lg:grid-cols-4', 'lg:grid-cols-5'];

interface Tile {
  /** The tab the tile opens. */
  tab: string;
  /** Tabs the tile shows as active. */
  tabs: string[];
  icon: IconDefinition;
  color: string;
  label: string;
  value: ReactNode;
}

function TileButton({
  tile,
  active,
  wide,
  onSelect,
}: {
  tile: Tile;
  active: boolean;
  /** Spans both columns of the two-column layout (the odd last tile). */
  wide: boolean;
  onSelect: (tab: string) => void;
}) {
  return (
    <button
      type='button'
      aria-pressed={active}
      className={classNames(
        'cursor-pointer rounded-md text-left focus-visible:outline-2 focus-visible:outline-(--mantine-primary-color-filled)',
        wide && 'col-span-2 lg:col-span-1',
      )}
      onClick={() => onSelect(tile.tab)}
    >
      <Card
        hoverable
        padding='md'
        className={classNames(
          'h-full transition-colors',
          active && 'border-(--mantine-primary-color-filled)! bg-(--mantine-primary-color-light)!',
        )}
      >
        <div className='flex items-center gap-3'>
          <ThemeIcon size={38} radius='md' variant='light' color={tile.color}>
            <FontAwesomeIcon icon={tile.icon} />
          </ThemeIcon>
          <div className='flex min-w-0 flex-col'>
            <span className='truncate text-xs font-semibold tracking-wide text-(--mantine-color-dimmed) uppercase'>
              {tile.label}
            </span>
            <span className='text-xl leading-tight font-semibold tabular-nums'>{tile.value}</span>
          </div>
        </div>
      </Card>
    </button>
  );
}

export function TilesSkeleton({ count = 4 }: { count?: number }) {
  return (
    <div className={classNames('grid grid-cols-2 gap-3', COLUMNS[Math.min(count, 4)])}>
      {Array.from({ length: count }, (_, index) => (
        <Card key={index} padding='md'>
          <div className='flex items-center gap-3'>
            <Skeleton height={38} width={38} radius='md' />
            <div className='flex flex-1 flex-col gap-2'>
              <Skeleton height={10} width='50%' />
              <Skeleton height={16} width='35%' />
            </div>
          </div>
        </Card>
      ))}
    </div>
  );
}

// the online count, the saved players and one count per list; bans and ip bans share a tile. Each tile opens
// its tab
export default function StatTiles({
  activeTab,
  online,
  withPlayers,
  onSelect,
}: {
  activeTab: string;
  online: OnlinePlayers | undefined;
  /** Whether there is a players tab (saved profiles). */
  withPlayers: boolean;
  onSelect: (tab: string) => void;
}) {
  const text = useText();
  const { game, overview, profiles } = usePlayerManager();
  const count = (kind: ListKind) => overview.lists[kind]?.length ?? 0;
  const hasBans = game.lists.some((spec) => spec.kind === 'bans');
  const hasIpBans = game.lists.some((spec) => spec.kind === 'ip_bans');
  const suffix = 'ml-1 text-sm font-normal text-(--mantine-color-dimmed)';

  const tiles: Tile[] = [];
  if (game.online) {
    const max = online?.max ?? overview.info.max_players;
    tiles.push({
      tab: 'online',
      tabs: ['online'],
      icon: faPlug,
      color: 'green',
      label: text('online.tab', {}),
      value: (
        <>
          {online?.count ?? '–'}
          {max !== null && <span className={suffix}>/ {max}</span>}
        </>
      ),
    });
  }
  if (withPlayers) {
    tiles.push({
      tab: 'players',
      tabs: ['players'],
      icon: faUsers,
      color: 'violet',
      label: text('players.tab', {}),
      value: profiles?.length ?? '–',
    });
  }
  for (const { kind } of game.lists) {
    if (kind === 'ip_bans' && hasBans) continue;
    if (kind === 'bans') {
      tiles.push({
        tab: 'bans',
        tabs: ['bans', 'ip_bans'],
        icon: faBan,
        color: LIST_STYLES.bans.color,
        label: hasIpBans ? text('tiles.banned', {}) : text('lists.bans.title', {}),
        value: (
          <>
            {count('bans')}
            {hasIpBans && count('ip_bans') > 0 && (
              <span className={suffix}>{text('tiles.ips', { count: count('ip_bans') })}</span>
            )}
          </>
        ),
      });
      continue;
    }
    tiles.push({
      tab: kind,
      tabs: [kind],
      icon: LIST_STYLES[kind].tab,
      color: LIST_STYLES[kind].color,
      label: text(`lists.${kind}.title`, {}),
      value: count(kind),
    });
  }

  return (
    <div className={classNames('grid grid-cols-2 gap-3', COLUMNS[Math.min(tiles.length, 5)])}>
      {tiles.map((tile, index) => (
        <TileButton
          key={tile.tab}
          tile={tile}
          active={tile.tabs.includes(activeTab)}
          wide={tiles.length % 2 === 1 && index === tiles.length - 1}
          onSelect={onSelect}
        />
      ))}
    </div>
  );
}
