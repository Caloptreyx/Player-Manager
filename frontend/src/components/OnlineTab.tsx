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
import { addBan, addOperator, addToWhitelist, kickPlayer, removeFromWhitelist, removeOperator } from '../api.ts';
import type { Access } from '../lib/access.ts';
import { matchesFilter, type OnlinePlayers, type Player, playerStatus, sortBy } from '../lib/players.ts';
import { useExtTranslations } from '../translations.ts';
import ListPanel from './ListPanel.tsx';
import PlayerRow from './PlayerRow.tsx';
import { usePlayerManager } from './playerManager.ts';
import RowAction from './RowAction.tsx';
import StatusBadges from './StatusBadges.tsx';

function OnlineRow({ player, onKicked }: { player: Player; onKicked: (name: string) => void }) {
  const { t: tExt } = useExtTranslations();
  const { serverUuid, edition, overview, listAccess, kickAccess, run, confirm } = usePlayerManager();
  const bedrock = edition === 'bedrock';
  const status = playerStatus(overview, player);
  const { name } = player;
  const id = player.id ?? undefined;
  // Bedrock members and visitors are in permissions.json too, but only operators count as opped
  const opped = status.operator !== null && status.operator.level !== 'member' && status.operator.level !== 'visitor';

  return (
    <PlayerRow
      player={player}
      badges={<StatusBadges status={status} />}
      actions={
        <>
          {status.whitelisted ? (
            <RowAction
              icon={faUserMinus}
              label={tExt(bedrock ? 'actions.allowlistRemove' : 'actions.whitelistRemove', {})}
              access={listAccess}
              onClick={() =>
                confirm({
                  title: tExt('confirm.removeTitle', { name }),
                  content: tExt(bedrock ? 'confirm.allowlistRemove' : 'confirm.whitelistRemove', { name }),
                  confirm: tExt('actions.remove', {}),
                  run: () => removeFromWhitelist(serverUuid, name),
                  success: tExt(bedrock ? 'toast.allowlistRemoved' : 'toast.whitelistRemoved', { name }),
                })
              }
            />
          ) : (
            <RowAction
              icon={faListCheck}
              label={tExt(bedrock ? 'actions.allowlistAdd' : 'actions.whitelistAdd', {})}
              access={listAccess}
              onClick={() =>
                run(
                  () => addToWhitelist(serverUuid, { name, id }),
                  tExt(bedrock ? 'toast.allowlistAdded' : 'toast.whitelistAdded', { name }),
                )
              }
            />
          )}

          {opped ? (
            <RowAction
              icon={faUserSlash}
              label={tExt('actions.deop', {})}
              access={listAccess}
              onClick={() =>
                confirm({
                  title: tExt('confirm.removeTitle', { name }),
                  content: tExt('confirm.operatorRemove', { name }),
                  confirm: tExt('actions.remove', {}),
                  run: () => removeOperator(serverUuid, { name, id }),
                  success: tExt('toast.deopped', { name }),
                })
              }
            />
          ) : (
            <RowAction
              icon={faUserShield}
              label={tExt('actions.op', {})}
              access={listAccess}
              disabledReason={bedrock && !id ? tExt('blockers.noXuid', {}) : null}
              onClick={() =>
                run(
                  () => addOperator(serverUuid, bedrock ? { name, id, level: 'operator' } : { name, id }),
                  tExt('toast.opped', { name }),
                )
              }
            />
          )}

          {!bedrock && !status.banned && (
            <RowAction
              icon={faBan}
              label={tExt('actions.ban', {})}
              access={listAccess}
              danger
              onClick={() =>
                confirm({
                  title: tExt('confirm.banTitle', { name }),
                  content: tExt('confirm.banContent', { name }),
                  confirm: tExt('actions.ban', {}),
                  withReason: true,
                  run: (reason) => addBan(serverUuid, { name, id, reason: reason || undefined }),
                  success: tExt('toast.banned', { name }),
                  onSuccess: () => onKicked(name),
                })
              }
            />
          )}

          <RowAction
            icon={faDoorOpen}
            label={tExt('actions.kick', {})}
            access={kickAccess}
            danger
            onClick={() =>
              confirm({
                title: tExt('confirm.kickTitle', { name }),
                content: tExt('confirm.kickContent', { name }),
                confirm: tExt('actions.kick', {}),
                withReason: true,
                run: (reason) => kickPlayer(serverUuid, { name, reason: reason || undefined }),
                success: tExt('toast.kicked', { name }),
                onSuccess: () => onKicked(name),
              })
            }
          />
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
  const { t: tExt } = useExtTranslations();
  const [filter, setFilter] = useState('');

  if (!access.visible) {
    return (
      <EmptyState flush icon={faPlug} title={tExt('tabs.online', {})} description={tExt('online.noPermission', {})} />
    );
  }
  if (access.blocker) {
    return (
      <EmptyState
        flush
        icon={faPlug}
        title={tExt('tabs.online', {})}
        description={tExt(access.blocker === 'transition' ? 'blockers.transition' : 'online.offline', {})}
      />
    );
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
      {tExt('common.refresh', {})}
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
        emptyText={tExt('online.empty', {})}
        info={
          <span className='flex flex-wrap items-center gap-x-3 gap-y-1'>
            <span className='font-medium text-(--mantine-color-text)'>
              {tExt('online.count', { count: online.count, max: online.max })}
            </span>
            <span className='inline-flex items-center gap-1'>
              <FontAwesomeIcon icon={faCircleInfo} />
              {tExt('online.hint', {})}
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
