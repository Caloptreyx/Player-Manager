import { faTrash, faUserShield } from '@fortawesome/free-solid-svg-icons';
import { useState } from 'react';
import Badge from '@/elements/data-display/Badge.tsx';
import { removeOperator } from '../api.ts';
import { matchesFilter, playerStatus, sortBy } from '../lib/players.ts';
import { useExtTranslations } from '../translations.ts';
import AddPlayerModal from './AddPlayerModal.tsx';
import ListPanel from './ListPanel.tsx';
import PlayerRow from './PlayerRow.tsx';
import { usePlayerManager } from './playerManager.ts';
import RowAction, { GatedButton } from './RowAction.tsx';
import StatusBadges, { LevelBadge } from './StatusBadges.tsx';

export default function OperatorsTab() {
  const { t: tExt } = useExtTranslations();
  const { serverUuid, overview, listAccess, confirm } = usePlayerManager();
  const [filter, setFilter] = useState('');
  const [adding, setAdding] = useState(false);

  const operators = sortBy(
    overview.operators,
    (operator) => operator.name,
    (operator) => operator.id,
  ).filter((operator) => matchesFilter([operator.name, operator.id], filter));

  return (
    <>
      <AddPlayerModal kind='operator' opened={adding} onClose={() => setAdding(false)} />
      <ListPanel
        filter={filter}
        onFilterChange={setFilter}
        total={overview.operators.length}
        emptyText={tExt('lists.emptyOperators', {})}
        actions={
          <GatedButton
            icon={faUserShield}
            label={tExt('actions.addOperator', {})}
            access={listAccess}
            onClick={() => setAdding(true)}
          />
        }
      >
        {operators.map((operator) => {
          const label = operator.name ?? operator.id ?? '';
          return (
            <PlayerRow
              key={`${operator.name}:${operator.id}`}
              player={operator}
              badges={
                <>
                  <LevelBadge level={operator.level} />
                  {operator.bypasses_player_limit && (
                    <Badge size='xs' variant='light' color='blue'>
                      {tExt('badges.bypassesLimit', {})}
                    </Badge>
                  )}
                  <StatusBadges status={playerStatus(overview, operator)} except='operator' />
                </>
              }
              actions={
                <RowAction
                  icon={faTrash}
                  label={tExt('actions.deop', {})}
                  access={listAccess}
                  danger
                  onClick={() =>
                    confirm({
                      title: tExt('confirm.removeTitle', { name: label }),
                      content: tExt('confirm.operatorRemove', { name: label }),
                      confirm: tExt('actions.remove', {}),
                      run: () =>
                        removeOperator(serverUuid, {
                          name: operator.name ?? undefined,
                          id: operator.id ?? undefined,
                        }),
                      success: tExt('toast.deopped', { name: label }),
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
