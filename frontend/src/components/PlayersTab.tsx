import { faChevronRight, faClockRotateLeft, faTriangleExclamation, faUsers } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Skeleton } from '@mantine/core';
import type { UseQueryResult } from '@tanstack/react-query';
import classNames from 'classnames';
import { useState } from 'react';
import { Link } from 'react-router';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import Avatar from '@/elements/data-display/Avatar.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import type { OnlinePlayers, ProfileSummary } from '../lib/model.ts';
import { matchesFilter, samePlayer } from '../lib/players.ts';
import ListPanel from './ListPanel.tsx';
import { usePlayerManager, useText } from './playerManager.ts';

function PlayersSkeleton({ count = 4 }: { count?: number }) {
  return (
    <div className='flex flex-col divide-y divide-(--mantine-color-default-border) rounded-md border border-(--mantine-color-default-border)'>
      {Array.from({ length: count }, (_, index) => (
        <div key={index} className='flex items-center gap-3 p-3'>
          <Skeleton height={36} width={36} radius='md' />
          <div className='flex flex-1 flex-col gap-2'>
            <Skeleton height={12} width='30%' />
            <Skeleton height={10} width='55%' />
          </div>
        </div>
      ))}
    </div>
  );
}

// everyone the server saved player data of, newest first; a row opens the player's profile
export default function PlayersTab({
  query,
  online,
}: {
  query: UseQueryResult<ProfileSummary[]>;
  /** The last online list, for the online badges; undefined while unknown. */
  online: OnlinePlayers | undefined;
}) {
  const text = useText();
  const { ui } = usePlayerManager();
  const [filter, setFilter] = useState('');

  if (query.isError) {
    return (
      <Alert color='red' icon={<FontAwesomeIcon icon={faTriangleExclamation} />} className='text-sm!'>
        <div className='flex flex-wrap items-center justify-between gap-2'>
          <span className='text-sm'>
            {text('players.loadError', {})} {httpErrorToHuman(query.error)}
          </span>
          <Button
            size='xs'
            variant='default'
            onClick={() => {
              query.refetch();
            }}
          >
            {text('common.retry', {})}
          </Button>
        </div>
      </Alert>
    );
  }
  if (!query.data) return <PlayersSkeleton />;

  const all = query.data;
  const players = all.filter((player) => matchesFilter([player.name, player.id], filter));

  return (
    <ListPanel
      filter={filter}
      onFilterChange={setFilter}
      searchPlaceholder={text('players.search', {})}
      total={all.length}
      shown={players.length}
      info={all.length > 0 && text('players.count', { count: all.length })}
      empty={
        <EmptyState
          flush
          icon={faUsers}
          title={text('players.emptyTitle', {})}
          description={text('players.empty', {})}
        />
      }
    >
      <ul className='flex flex-col divide-y divide-(--mantine-color-default-border) overflow-hidden rounded-md border border-(--mantine-color-default-border)'>
        {players.map((player) => {
          const isOnline = online?.players.some((entry) => samePlayer(entry, player)) ?? false;
          return (
            <li key={player.id}>
              <Link
                to={encodeURIComponent(player.id)}
                relative='path'
                className='flex items-center gap-3 p-3 transition-colors hover:bg-(--mantine-color-default-hover) focus-visible:bg-(--mantine-color-default-hover) focus-visible:outline-none'
              >
                <Avatar
                  size={36}
                  radius='md'
                  className='shrink-0'
                  src={ui.avatarUrl(player, 72)}
                  name={player.name ?? '?'}
                />
                <span className='flex min-w-0 flex-1 flex-col'>
                  <span
                    className={classNames(
                      'truncate text-sm',
                      player.name === null ? 'text-(--mantine-color-dimmed) italic' : 'font-medium',
                    )}
                  >
                    {player.name ?? text('common.unknownPlayer', {})}
                  </span>
                  <span className='truncate font-mono text-xs text-(--mantine-color-dimmed)'>{player.id}</span>
                  <span className='text-xs text-(--mantine-color-dimmed) sm:hidden'>
                    <FontAwesomeIcon icon={faClockRotateLeft} className='mr-1' />
                    <FormattedTimestamp timestamp={player.last_saved} withTooltip={false} />
                  </span>
                </span>
                {isOnline && (
                  <Badge variant='light' color='green' className='shrink-0'>
                    {text('players.online', {})}
                  </Badge>
                )}
                <span
                  className='hidden shrink-0 text-xs text-(--mantine-color-dimmed) sm:inline'
                  title={text('profile.lastSaved', {})}
                >
                  <FontAwesomeIcon icon={faClockRotateLeft} className='mr-1' />
                  <FormattedTimestamp timestamp={player.last_saved} />
                </span>
                <FontAwesomeIcon icon={faChevronRight} className='shrink-0 text-xs text-(--mantine-color-dimmed)' />
              </Link>
            </li>
          );
        })}
      </ul>
    </ListPanel>
  );
}
