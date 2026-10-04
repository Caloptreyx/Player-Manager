import { faBan, faNetworkWired, faUnlock } from '@fortawesome/free-solid-svg-icons';
import { useState } from 'react';
import { removeIpBan } from '../api.ts';
import { matchesFilter, sortBy } from '../lib/players.ts';
import { useExtTranslations } from '../translations.ts';
import AddIpBanModal from './AddIpBanModal.tsx';
import BanDetails from './BanDetails.tsx';
import ListPanel from './ListPanel.tsx';
import PlayerRow from './PlayerRow.tsx';
import { usePlayerManager } from './playerManager.ts';
import RowAction, { GatedButton } from './RowAction.tsx';

export default function IpBansTab() {
  const { t: tExt } = useExtTranslations();
  const { serverUuid, overview, listAccess, confirm } = usePlayerManager();
  const [filter, setFilter] = useState('');
  const [adding, setAdding] = useState(false);

  const bans = sortBy(overview.ip_bans, (ban) => ban.ip).filter((ban) => matchesFilter([ban.ip, ban.reason], filter));

  return (
    <>
      <AddIpBanModal opened={adding} onClose={() => setAdding(false)} />
      <ListPanel
        filter={filter}
        onFilterChange={setFilter}
        total={overview.ip_bans.length}
        emptyText={tExt('lists.emptyIpBans', {})}
        actions={
          <GatedButton
            icon={faBan}
            label={tExt('actions.banIp', {})}
            access={listAccess}
            onClick={() => setAdding(true)}
          />
        }
      >
        {bans.map((ban) => (
          <PlayerRow
            key={ban.ip}
            icon={faNetworkWired}
            title={ban.ip}
            details={<BanDetails ban={ban} />}
            actions={
              <RowAction
                icon={faUnlock}
                label={tExt('actions.pardon', {})}
                access={listAccess}
                danger
                onClick={() =>
                  confirm({
                    title: tExt('confirm.removeTitle', { name: ban.ip }),
                    content: tExt('confirm.ipBanRemove', { ip: ban.ip }),
                    confirm: tExt('actions.pardon', {}),
                    run: () => removeIpBan(serverUuid, ban.ip),
                    success: tExt('toast.ipUnbanned', { ip: ban.ip }),
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
