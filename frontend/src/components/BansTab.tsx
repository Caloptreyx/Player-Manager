import { faBan, faUnlock } from '@fortawesome/free-solid-svg-icons';
import { useState } from 'react';
import { removeBan } from '../api.ts';
import { matchesFilter, playerStatus, sortBy } from '../lib/players.ts';
import { useExtTranslations } from '../translations.ts';
import AddPlayerModal from './AddPlayerModal.tsx';
import BanDetails from './BanDetails.tsx';
import ListPanel from './ListPanel.tsx';
import PlayerRow from './PlayerRow.tsx';
import { usePlayerManager } from './playerManager.ts';
import RowAction, { GatedButton } from './RowAction.tsx';
import StatusBadges from './StatusBadges.tsx';

export default function BansTab() {
  const { t: tExt } = useExtTranslations();
  const { serverUuid, overview, listAccess, confirm } = usePlayerManager();
  const [filter, setFilter] = useState('');
  const [adding, setAdding] = useState(false);

  const bans = sortBy(overview.bans, (ban) => ban.name).filter((ban) =>
    matchesFilter([ban.name, ban.id, ban.reason], filter),
  );

  return (
    <>
      <AddPlayerModal kind='ban' opened={adding} onClose={() => setAdding(false)} />
      <ListPanel
        filter={filter}
        onFilterChange={setFilter}
        total={overview.bans.length}
        emptyText={tExt('lists.emptyBans', {})}
        actions={
          <GatedButton
            icon={faBan}
            label={tExt('actions.banPlayer', {})}
            access={listAccess}
            onClick={() => setAdding(true)}
          />
        }
      >
        {bans.map((ban) => (
          <PlayerRow
            key={`${ban.name}:${ban.id}`}
            player={ban}
            badges={<StatusBadges status={playerStatus(overview, ban)} except='banned' />}
            details={<BanDetails ban={ban} />}
            actions={
              <RowAction
                icon={faUnlock}
                label={tExt('actions.pardon', {})}
                access={listAccess}
                danger
                onClick={() =>
                  confirm({
                    title: tExt('confirm.removeTitle', { name: ban.name }),
                    content: tExt('confirm.banRemove', { name: ban.name }),
                    confirm: tExt('actions.pardon', {}),
                    run: () => removeBan(serverUuid, ban.name),
                    success: tExt('toast.unbanned', { name: ban.name }),
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
