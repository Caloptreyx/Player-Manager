import { faNetworkWired } from '@fortawesome/free-solid-svg-icons';
import { useState } from 'react';
import Badge from '@/elements/data-display/Badge.tsx';
import Table, { TableData, TableRow } from '@/elements/data-display/Table.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import { usePageBreakpoint } from '@/plugins/viewport/usePageBreakpoint.ts';
import { removeFromList } from '../api.ts';
import { removeBody } from '../lib/lists.ts';
import type { Entry, ListKind, ListSpec, Overview } from '../lib/model.ts';
import { matchesFilter, playerStatus, sortBy } from '../lib/players.ts';
import AddEntryModal from './AddEntryModal.tsx';
import BanDetails from './BanDetails.tsx';
import ListPanel from './ListPanel.tsx';
import { LIST_STYLES } from './listStyles.ts';
import PlayerCell from './PlayerCell.tsx';
import { usePlayerActions } from './playerActions.ts';
import { usePlayerManager, useText } from './playerManager.ts';
import RowAction, { ActionMenu, GatedButton } from './RowAction.tsx';
import StatusBadges, { LevelBadge } from './StatusBadges.tsx';

const NONE = <span className='text-sm text-(--mantine-color-dimmed)'>–</span>;

// the cells of one entry, shared by the desktop table row and the stacked mobile card
const hasDetails = (entry: Entry) =>
  entry.level !== null || entry.bypasses_player_limit || entry.reason || entry.source || entry.created || entry.expires;

/** Whether the player of the entry is on any other player list, which shows as status badges. */
const hasOtherLists = (lists: Overview['lists'], entry: Entry, kind: ListKind) =>
  Object.keys(playerStatus(lists, entry)).some((other) => other !== kind);

function useEntryCells(spec: ListSpec, entry: Entry) {
  const text = useText();
  const { serverUuid, overview, editAccess, confirm } = usePlayerManager();
  const { kind } = spec;
  const byIp = spec.target === 'ip';
  const label = entry.ip ?? entry.name ?? entry.id ?? '';
  const others = usePlayerActions(entry, { except: kind });

  const player = byIp ? <PlayerCell icon={faNetworkWired} title={entry.ip ?? ''} /> : <PlayerCell player={entry} />;

  // the badges of the other lists the player is on; null when there are none
  const status =
    !byIp && hasOtherLists(overview.lists, entry, kind) ? (
      <StatusBadges status={playerStatus(overview.lists, entry)} except={kind} />
    ) : null;

  const hasBan = entry.reason || entry.source || entry.created || entry.expires;
  const details = hasDetails(entry) ? (
    <div className='flex flex-col items-start gap-1'>
      {(entry.level !== null || entry.bypasses_player_limit) && (
        <div className='flex flex-wrap gap-1'>
          {entry.level !== null && <LevelBadge level={entry.level} />}
          {entry.bypasses_player_limit && (
            <Badge size='sm' variant='light' color='blue'>
              {text('badges.bypassesLimit', {})}
            </Badge>
          )}
        </div>
      )}
      {hasBan && <BanDetails entry={entry} />}
    </div>
  ) : null;

  const actions = (
    <div className='flex items-center justify-end gap-0.5'>
      <RowAction
        icon={LIST_STYLES[kind].remove}
        label={text(`lists.${kind}.remove`, {})}
        access={editAccess}
        danger
        onClick={() =>
          confirm({
            title: text('lists.removeTitle', { name: label }),
            content: text(`lists.${kind}.removeContent`, { name: label }),
            confirm: text(`lists.${kind}.removeConfirm`, {}),
            run: () => removeFromList(serverUuid, kind, removeBody(spec, entry)),
            success: text(`lists.${kind}.removed`, { name: label }),
          })
        }
      />
      {!byIp && <ActionMenu actions={others} reserve />}
    </div>
  );

  return { player, status, details, actions };
}

function EntryRow({
  spec,
  entry,
  withStatus,
  withDetails,
}: {
  spec: ListSpec;
  entry: Entry;
  withStatus: boolean;
  withDetails: boolean;
}) {
  const { player, status, details, actions } = useEntryCells(spec, entry);

  return (
    <TableRow className='transition-colors hover:bg-(--mantine-color-default-hover)'>
      <TableData>{player}</TableData>
      {withStatus && (
        <TableData>
          <div className='flex flex-wrap gap-1'>{status ?? NONE}</div>
        </TableData>
      )}
      {withDetails && <TableData className='text-wrap! min-w-48'>{details ?? NONE}</TableData>}
      <TableData className='w-px'>{actions}</TableData>
    </TableRow>
  );
}

function EntryCard({ spec, entry }: { spec: ListSpec; entry: Entry }) {
  const { player, status, details, actions } = useEntryCells(spec, entry);

  return (
    <div className='flex flex-col gap-2 rounded-md border border-(--mantine-color-default-border) p-3'>
      <div className='flex items-center gap-2'>
        <div className='min-w-0 flex-1'>{player}</div>
        {actions}
      </div>
      {(status || details) && (
        <div className='flex flex-col items-start gap-1.5 pl-11'>
          {status && <div className='flex flex-wrap gap-1'>{status}</div>}
          {details}
        </div>
      )}
    </div>
  );
}

// one list of the game, rendered from its spec: a table on wide screens, stacked cards on small ones
export default function ListTab({ spec }: { spec: ListSpec }) {
  const text = useText();
  const { overview, editAccess } = usePlayerManager();
  const [filter, setFilter] = useState('');
  const [adding, setAdding] = useState(false);
  const wide = usePageBreakpoint('lg');
  const { kind } = spec;
  const style = LIST_STYLES[kind];
  const all = overview.lists[kind] ?? [];
  const byIp = spec.target === 'ip';
  // columns nobody on the list has anything for (details of a Java whitelist) are left out
  const withStatus = !byIp && all.some((entry) => hasOtherLists(overview.lists, entry, kind));
  const withDetails = all.some(hasDetails);

  const entries = sortBy(
    all,
    (entry) => (byIp ? entry.ip : entry.name),
    (entry) => entry.id,
  ).filter((entry) => matchesFilter([entry.ip, entry.name, entry.id, entry.reason], filter));

  const addButton = (
    <GatedButton
      icon={style.add}
      label={text(`lists.${kind}.add`, {})}
      access={editAccess}
      onClick={() => setAdding(true)}
    />
  );

  return (
    <>
      <AddEntryModal spec={spec} opened={adding} onClose={() => setAdding(false)} />
      <ListPanel
        filter={filter}
        onFilterChange={setFilter}
        total={all.length}
        shown={entries.length}
        actions={addButton}
        empty={
          <EmptyState
            flush
            icon={style.tab}
            title={text(`lists.${kind}.empty`, {})}
            description={text(`lists.${kind}.emptyDescription`, {})}
          >
            {addButton}
          </EmptyState>
        }
      >
        {wide ? (
          <Table
            columns={[
              { name: text(byIp ? 'lists.address' : 'lists.player', {}), className: 'w-2/5' },
              ...(withStatus ? [text('lists.status', {})] : []),
              ...(withDetails ? [text('lists.details', {})] : []),
              { name: text('lists.actions', {}), className: 'text-right [&>div]:justify-end' },
            ]}
          >
            {entries.map((entry) => (
              <EntryRow
                key={`${entry.ip}:${entry.name}:${entry.id}`}
                spec={spec}
                entry={entry}
                withStatus={withStatus}
                withDetails={withDetails}
              />
            ))}
          </Table>
        ) : (
          <div className='flex flex-col gap-2'>
            {entries.map((entry) => (
              <EntryCard key={`${entry.ip}:${entry.name}:${entry.id}`} spec={spec} entry={entry} />
            ))}
          </div>
        )}
      </ListPanel>
    </>
  );
}
