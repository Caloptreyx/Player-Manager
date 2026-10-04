import { faTrash, faUserPlus } from '@fortawesome/free-solid-svg-icons';
import { useState } from 'react';
import Badge from '@/elements/data-display/Badge.tsx';
import { removeFromWhitelist } from '../api.ts';
import { matchesFilter, playerStatus, sortBy } from '../lib/players.ts';
import { useExtTranslations } from '../translations.ts';
import AddPlayerModal from './AddPlayerModal.tsx';
import ListPanel from './ListPanel.tsx';
import PlayerRow from './PlayerRow.tsx';
import { usePlayerManager } from './playerManager.ts';
import RowAction, { GatedButton } from './RowAction.tsx';
import StatusBadges from './StatusBadges.tsx';

export default function WhitelistTab() {
  const { t: tExt } = useExtTranslations();
  const { serverUuid, edition, overview, listAccess, confirm } = usePlayerManager();
  const [filter, setFilter] = useState('');
  const [adding, setAdding] = useState(false);
  const bedrock = edition === 'bedrock';

  const entries = sortBy(overview.whitelist, (entry) => entry.name).filter((entry) =>
    matchesFilter([entry.name, entry.id], filter),
  );

  return (
    <>
      <AddPlayerModal kind='whitelist' opened={adding} onClose={() => setAdding(false)} />
      <ListPanel
        filter={filter}
        onFilterChange={setFilter}
        total={overview.whitelist.length}
        emptyText={tExt(bedrock ? 'lists.emptyAllowlist' : 'lists.emptyWhitelist', {})}
        actions={
          <GatedButton
            icon={faUserPlus}
            label={tExt('actions.addPlayer', {})}
            access={listAccess}
            onClick={() => setAdding(true)}
          />
        }
      >
        {entries.map((entry) => (
          <PlayerRow
            key={`${entry.name}:${entry.id}`}
            player={entry}
            badges={
              <>
                <StatusBadges status={playerStatus(overview, entry)} except='whitelisted' />
                {entry.ignores_player_limit && (
                  <Badge size='xs' variant='light' color='blue'>
                    {tExt('badges.ignoresLimit', {})}
                  </Badge>
                )}
              </>
            }
            actions={
              <RowAction
                icon={faTrash}
                label={tExt('actions.remove', {})}
                access={listAccess}
                danger
                onClick={() =>
                  confirm({
                    title: tExt('confirm.removeTitle', { name: entry.name }),
                    content: tExt(bedrock ? 'confirm.allowlistRemove' : 'confirm.whitelistRemove', {
                      name: entry.name,
                    }),
                    confirm: tExt('actions.remove', {}),
                    run: () => removeFromWhitelist(serverUuid, entry.name),
                    success: tExt(bedrock ? 'toast.allowlistRemoved' : 'toast.whitelistRemoved', { name: entry.name }),
                  })
                }
              />
            }
          />
        ))}
      </ListPanel>
    </>
  );
}
