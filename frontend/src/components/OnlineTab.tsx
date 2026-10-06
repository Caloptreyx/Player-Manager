import {
  faArrowRotateRight,
  faEyeSlash,
  faPlug,
  faPowerOff,
  faTerminal,
  faTriangleExclamation,
  faUserClock,
  faWifi,
} from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Skeleton } from '@mantine/core';
import type { UseQueryResult } from '@tanstack/react-query';
import { useState } from 'react';
import { getHttpStatus, httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import type { Access } from '../lib/access.ts';
import type { OnlinePlayers, Player } from '../lib/model.ts';
import { matchesFilter, playerStatus, sortBy } from '../lib/players.ts';
import ListPanel from './ListPanel.tsx';
import PlayerCell from './PlayerCell.tsx';
import { usePlayerActions } from './playerActions.ts';
import { usePlayerManager, useText } from './playerManager.ts';
import { ActionMenu } from './RowAction.tsx';
import StatusBadges from './StatusBadges.tsx';

const GRID = 'grid grid-cols-1 gap-3 sm:grid-cols-2 xl:grid-cols-3';

function OnlineCard({ player, onKicked }: { player: Player; onKicked: (name: string) => void }) {
  const { overview } = usePlayerManager();
  const actions = usePlayerActions(player, { kick: true, onGone: onKicked });
  const status = playerStatus(overview.lists, player);

  return (
    <div className='flex items-start gap-2 rounded-md border border-(--mantine-color-default-border) p-3 transition-colors hover:bg-(--mantine-color-default-hover)'>
      <div className='flex min-w-0 flex-1 flex-col gap-2'>
        <PlayerCell player={player} size={48} />
        {Object.keys(status).length > 0 && (
          <div className='flex flex-wrap gap-1 pl-15'>
            <StatusBadges status={status} />
          </div>
        )}
      </div>
      <ActionMenu actions={actions} />
    </div>
  );
}

function OnlineSkeleton({ count = 3 }: { count?: number }) {
  return (
    <div className={GRID}>
      {Array.from({ length: count }, (_, index) => (
        <div
          key={index}
          className='flex items-center gap-3 rounded-md border border-(--mantine-color-default-border) p-3'
        >
          <Skeleton height={48} width={48} radius='md' />
          <div className='flex flex-1 flex-col gap-2'>
            <Skeleton height={12} width='45%' />
            <Skeleton height={10} width='75%' />
          </div>
        </div>
      ))}
    </div>
  );
}

/** Where the list came from; only a console answer cost a `list` command, the others refresh by themselves. */
function SourceChip({ source }: { source: OnlinePlayers['source'] }) {
  const text = useText();
  const viaConsole = source === 'console';

  return (
    <Tooltip label={text(viaConsole ? 'online.hint' : 'online.autoRefresh', {})} multiline maw={260}>
      <Badge
        size='sm'
        variant='default'
        color='gray'
        className='cursor-help font-normal normal-case'
        leftSection={<FontAwesomeIcon icon={viaConsole ? faTerminal : faWifi} />}
      >
        {text(`online.source.${source}`, {})}
      </Badge>
    </Tooltip>
  );
}

// the online list is fetched when the tab opens and on refresh; answers that did not need the console refresh
// themselves in the background (see `useOnlinePlayers`) without the loading skeleton
export default function OnlineTab({
  access,
  query,
  onKicked,
}: {
  access: Access;
  query: UseQueryResult<OnlinePlayers>;
  onKicked: (name: string) => void;
}) {
  const text = useText();
  const [filter, setFilter] = useState('');
  const [refreshing, setRefreshing] = useState(false);

  if (!access.visible || access.blocker) {
    const offline = access.visible && access.blocker === 'notRunning';
    return (
      <EmptyState
        flush
        icon={offline ? faPowerOff : faPlug}
        title={text(offline ? 'online.offlineTitle' : 'online.unavailableTitle', {})}
        description={
          !access.visible
            ? text('online.noPermission', {})
            : offline
              ? text('online.offline', {})
              : text(`blockers.${access.blocker ?? 'noPermission'}`, {})
        }
      />
    );
  }

  const refetch = () => {
    setRefreshing(true);
    query.refetch().finally(() => setRefreshing(false));
  };

  // 504: no source answered in time, which is worth a retry rather than alarming
  const error = query.isError && !query.isFetching && (
    <Alert
      color={getHttpStatus(query.error) === 504 ? 'yellow' : 'red'}
      icon={<FontAwesomeIcon icon={faTriangleExclamation} />}
      className='text-sm!'
    >
      <div className='flex flex-wrap items-center justify-between gap-2'>
        <span className='text-sm'>{httpErrorToHuman(query.error)}</span>
        <Button size='xs' variant='default' onClick={refetch}>
          {text('common.retry', {})}
        </Button>
      </div>
    </Alert>
  );

  if (!query.data && !query.isFetching) return error || <OnlineSkeleton />;

  const online = query.data;
  // the skeleton stands in for the first answer and explicit refreshes, not for the silent background ones
  const loading = !online || refreshing;
  const players = online
    ? sortBy(online.players, (player) => player.name).filter((player) =>
        matchesFilter([player.name, player.id], filter),
      )
    : [];
  const hidden = online && !online.complete ? Math.max(0, online.count - online.players.length) : 0;
  const hiddenNote = hidden === 1 ? text('online.hiddenOne', {}) : text('online.hidden', { count: hidden });
  const checked = new Date(query.dataUpdatedAt).toLocaleTimeString(undefined, {
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hour12: false,
  });

  return (
    <div className='flex flex-col gap-3'>
      {error}
      <ListPanel
        filter={filter}
        onFilterChange={setFilter}
        searchPlaceholder={text('players.search', {})}
        total={online?.players.length ?? 0}
        shown={players.length}
        info={
          <span className='inline-flex flex-wrap items-center gap-x-1.5 gap-y-1'>
            {online ? (
              <>
                <span className='font-medium text-(--mantine-color-text)'>
                  {text('online.count', { count: online.count, max: online.max })}
                </span>
                <span>·</span>
                <span>{text('online.checked', { time: checked })}</span>
                <span className='ml-1'>
                  <SourceChip source={online.source} />
                </span>
              </>
            ) : (
              <Skeleton height={12} width={180} />
            )}
          </span>
        }
        actions={
          <Button
            variant='default'
            leftSection={<FontAwesomeIcon icon={faArrowRotateRight} />}
            loading={loading && query.isFetching}
            onClick={refetch}
          >
            {text('common.refresh', {})}
          </Button>
        }
        empty={
          loading ? (
            <OnlineSkeleton />
          ) : (
            <EmptyState
              flush
              icon={hidden > 0 ? faEyeSlash : faUserClock}
              title={hidden > 0 ? hiddenNote : text('online.emptyTitle', {})}
              description={hidden > 0 ? '' : text('online.empty', {})}
            />
          )
        }
      >
        {loading ? (
          <OnlineSkeleton count={Math.min(Math.max(players.length, 1), 6)} />
        ) : (
          <div className={GRID}>
            {players.map((player) => (
              <OnlineCard key={player.name} player={player} onKicked={onKicked} />
            ))}
          </div>
        )}
      </ListPanel>
      {hidden > 0 && !loading && (online?.players.length ?? 0) > 0 && (
        <p className='flex items-center gap-2 text-sm text-(--mantine-color-dimmed)'>
          <FontAwesomeIcon icon={faEyeSlash} />
          {hiddenNote}
        </p>
      )}
    </div>
  );
}
