import {
  faArrowRotateRight,
  faBan,
  faCircleInfo,
  faDoorOpen,
  faListCheck,
  faPlug,
  faUserMinus,
  faUserShield,
  faUserSlash,
} from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import type { UseQueryResult } from '@tanstack/react-query';
import { useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import { addToList, kickPlayer, removeFromList } from '../api.ts';
import type { Access } from '../lib/access.ts';
import { addBody, removeBody } from '../lib/lists.ts';
import type { ListKind, OnlinePlayers, Player } from '../lib/model.ts';
import { matchesFilter, playerStatus, sortBy } from '../lib/players.ts';
import ListPanel from './ListPanel.tsx';
import PlayerRow from './PlayerRow.tsx';
import { usePlayerManager, useText } from './playerManager.ts';
import RowAction from './RowAction.tsx';
import StatusBadges from './StatusBadges.tsx';

// the actions of a row follow what the game supports: whitelist and operator toggles, ban, kick
function OnlineRow({ player, onKicked }: { player: Player; onKicked: (name: string) => void }) {
  const text = useText();
  const { serverUuid, game, ui, overview, editAccess, kickAccess, run, confirm } = usePlayerManager();
  const status = playerStatus(overview.lists, player);
  const { name, id } = player;
  const spec = (kind: ListKind) => game.lists.find((list) => list.kind === kind);
  const whitelist = spec('whitelist');
  const operators = spec('operators');
  const bans = spec('bans');
  const { kick } = game;
  const { whitelist: listed, operators: operator, bans: banned } = status;
  // Bedrock members and visitors are in permissions.json too, but only operators count as opped
  const opped = operator !== undefined && (operator.level === null || ui.isOperatorLevel(operator.level));

  return (
    <PlayerRow
      player={player}
      badges={<StatusBadges status={status} />}
      actions={
        <>
          {whitelist &&
            (listed ? (
              <RowAction
                icon={faUserMinus}
                label={text('lists.whitelist.remove', {})}
                access={editAccess}
                onClick={() =>
                  confirm({
                    title: text('lists.removeTitle', { name }),
                    content: text('lists.whitelist.removeContent', { name }),
                    confirm: text('lists.whitelist.removeConfirm', {}),
                    run: () => removeFromList(serverUuid, 'whitelist', removeBody(whitelist, listed)),
                    success: text('lists.whitelist.removed', { name }),
                  })
                }
              />
            ) : (
              <RowAction
                icon={faListCheck}
                label={text('lists.whitelist.addOnline', {})}
                access={editAccess}
                onClick={() =>
                  run(
                    () => addToList(serverUuid, 'whitelist', addBody(whitelist, { name, id })),
                    text('lists.whitelist.added', { name }),
                  )
                }
              />
            ))}

          {operators &&
            (opped ? (
              <RowAction
                icon={faUserSlash}
                label={text('lists.operators.remove', {})}
                access={editAccess}
                onClick={() =>
                  confirm({
                    title: text('lists.removeTitle', { name }),
                    content: text('lists.operators.removeContent', { name }),
                    confirm: text('lists.operators.removeConfirm', {}),
                    run: () => removeFromList(serverUuid, 'operators', removeBody(operators, operator)),
                    success: text('lists.operators.removed', { name }),
                  })
                }
              />
            ) : (
              <RowAction
                icon={faUserShield}
                label={text('lists.operators.addOnline', {})}
                access={editAccess}
                onClick={() =>
                  run(
                    () => addToList(serverUuid, 'operators', addBody(operators, { name, id })),
                    text('lists.operators.added', { name }),
                  )
                }
              />
            ))}

          {bans && !banned && (
            <RowAction
              icon={faBan}
              label={text('lists.bans.addOnline', {})}
              access={editAccess}
              danger
              onClick={() =>
                confirm({
                  title: text('online.banTitle', { name }),
                  content: text('online.banContent', { name }),
                  confirm: text('lists.bans.addOnline', {}),
                  withReason: bans.reason,
                  run: (reason) => addToList(serverUuid, 'bans', addBody(bans, { name, id, reason })),
                  success: text('lists.bans.added', { name }),
                  onSuccess: () => onKicked(name),
                })
              }
            />
          )}

          {kick && (
            <RowAction
              icon={faDoorOpen}
              label={text('online.kick', {})}
              access={kickAccess}
              danger
              onClick={() =>
                confirm({
                  title: text('online.kickTitle', { name }),
                  content: text('online.kickContent', { name }),
                  confirm: text('online.kick', {}),
                  withReason: kick.reason,
                  run: (reason) => kickPlayer(serverUuid, { name, reason: reason || undefined }),
                  success: text('online.kicked', { name }),
                  onSuccess: () => onKicked(name),
                })
              }
            />
          )}
        </>
      }
    />
  );
}

// the online list is fetched on demand only: every fetch runs `list` in the server console
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

  if (!access.visible || access.blocker) {
    const description = !access.visible
      ? text('online.noPermission', {})
      : access.blocker === 'notRunning'
        ? text('online.offline', {})
        : text(`blockers.${access.blocker ?? 'noPermission'}`, {});
    return <EmptyState flush icon={faPlug} title={text('online.tab', {})} description={description} />;
  }

  const refresh = (
    <Button
      size='xs'
      variant='default'
      leftSection={<FontAwesomeIcon icon={faArrowRotateRight} />}
      loading={query.isFetching}
      onClick={() => {
        query.refetch();
      }}
    >
      {text('common.refresh', {})}
    </Button>
  );

  if (!query.data) {
    return query.isError ? (
      <Alert color='red' title={httpErrorToHuman(query.error)}>
        <div className='mt-2'>{refresh}</div>
      </Alert>
    ) : (
      <Spinner.Centered />
    );
  }

  const online = query.data;
  const players = sortBy(online.players, (player) => player.name).filter((player) =>
    matchesFilter([player.name, player.id], filter),
  );

  return (
    <div className='flex flex-col gap-2'>
      {query.isError && <Alert color='red'>{httpErrorToHuman(query.error)}</Alert>}
      <ListPanel
        filter={filter}
        onFilterChange={setFilter}
        total={online.players.length}
        emptyText={text('online.empty', {})}
        info={
          <span className='flex flex-wrap items-center gap-x-3 gap-y-1'>
            <span className='font-medium text-(--mantine-color-text)'>
              {text('online.count', { count: online.count, max: online.max })}
            </span>
            <span className='inline-flex items-center gap-1'>
              <FontAwesomeIcon icon={faCircleInfo} />
              {text('online.hint', {})}
            </span>
          </span>
        }
        actions={refresh}
      >
        {players.map((player) => (
          <OnlineRow key={player.name} player={player} onKicked={onKicked} />
        ))}
      </ListPanel>
    </div>
  );
}
