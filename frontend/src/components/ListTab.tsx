import { faNetworkWired } from '@fortawesome/free-solid-svg-icons';
import { useState } from 'react';
import Badge from '@/elements/data-display/Badge.tsx';
import { removeFromList } from '../api.ts';
import { removeBody } from '../lib/lists.ts';
import type { ListSpec } from '../lib/model.ts';
import { matchesFilter, playerStatus, sortBy } from '../lib/players.ts';
import AddEntryModal from './AddEntryModal.tsx';
import BanDetails from './BanDetails.tsx';
import ListPanel from './ListPanel.tsx';
import { LIST_STYLES } from './listStyles.ts';
import PlayerRow from './PlayerRow.tsx';
import { usePlayerManager, useText } from './playerManager.ts';
import RowAction, { GatedButton } from './RowAction.tsx';
import StatusBadges, { LevelBadge } from './StatusBadges.tsx';

// one list of the game, rendered from its spec: player rows with level, limit and cross-reference badges, or
// ip rows; ban details wherever the entries carry them
export default function ListTab({ spec }: { spec: ListSpec }) {
  const text = useText();
  const { serverUuid, overview, editAccess, confirm } = usePlayerManager();
  const [filter, setFilter] = useState('');
  const [adding, setAdding] = useState(false);
  const { kind } = spec;
  const style = LIST_STYLES[kind];
  const all = overview.lists[kind] ?? [];

  const entries = sortBy(
    all,
    (entry) => (spec.target === 'ip' ? entry.ip : entry.name),
    (entry) => entry.id,
  ).filter((entry) => matchesFilter([entry.ip, entry.name, entry.id, entry.reason], filter));

  return (
    <>
      <AddEntryModal spec={spec} opened={adding} onClose={() => setAdding(false)} />
      <ListPanel
        filter={filter}
        onFilterChange={setFilter}
        total={all.length}
        emptyText={text(`lists.${kind}.empty`, {})}
        actions={
          <GatedButton
            icon={style.add}
            label={text(`lists.${kind}.add`, {})}
            access={editAccess}
            onClick={() => setAdding(true)}
          />
        }
      >
        {entries.map((entry) => {
          const label = entry.ip ?? entry.name ?? entry.id ?? '';
          return (
            <PlayerRow
              key={`${entry.ip}:${entry.name}:${entry.id}`}
              player={spec.target === 'player' ? entry : undefined}
              icon={spec.target === 'ip' ? faNetworkWired : undefined}
              title={entry.ip ?? undefined}
              badges={
                spec.target === 'player' && (
                  <>
                    {entry.level !== null && <LevelBadge level={entry.level} />}
                    {entry.bypasses_player_limit && (
                      <Badge size='xs' variant='light' color='blue'>
                        {text('badges.bypassesLimit', {})}
                      </Badge>
                    )}
                    <StatusBadges status={playerStatus(overview.lists, entry)} except={kind} />
                  </>
                )
              }
              details={(entry.reason || entry.source || entry.created || entry.expires) && <BanDetails entry={entry} />}
              actions={
                <RowAction
                  icon={style.remove}
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
              }
            />
          );
        })}
      </ListPanel>
    </>
  );
}
