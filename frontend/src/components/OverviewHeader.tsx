import { faArrowRotateRight, faShieldHalved, faTriangleExclamation } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useState } from 'react';
import ActionIcon from '@/elements/buttons/ActionIcon.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Card from '@/elements/data-display/Card.tsx';
import Switch from '@/elements/input/Switch.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import { setWhitelistEnabled } from '../api.ts';
import type { Access } from '../lib/access.ts';
import type { ServerState } from '../lib/players.ts';
import { useExtTranslations } from '../translations.ts';
import { usePlayerManager } from './playerManager.ts';

const STATE_COLORS: Record<ServerState, string> = {
  offline: 'red',
  starting: 'yellow',
  stopping: 'yellow',
  running: 'green',
};

// edition, server state, online mode and the whitelist switch, above the tabs
export default function OverviewHeader({
  toggleAccess,
  refreshing,
  onRefresh,
}: {
  toggleAccess: Access;
  refreshing: boolean;
  onRefresh: () => void;
}) {
  const { t: tExt } = useExtTranslations();
  const { serverUuid, edition, overview, run } = usePlayerManager();
  const [toggling, setToggling] = useState(false);
  const bedrock = edition === 'bedrock';
  const enabled = overview.whitelist_enabled ?? false;

  const toggle = async (next: boolean) => {
    setToggling(true);
    const toast = bedrock
      ? tExt(next ? 'toast.allowlistOn' : 'toast.allowlistOff', {})
      : tExt(next ? 'toast.whitelistOn' : 'toast.whitelistOff', {});
    await run(() => setWhitelistEnabled(serverUuid, next), toast);
    setToggling(false);
  };

  const blocker = toggleAccess.visible
    ? toggleAccess.blocker && tExt(`blockers.${toggleAccess.blocker}`, {})
    : tExt('blockers.noPermission', {});
  const whitelistSwitch = (
    <Switch
      label={tExt(bedrock ? 'header.allowlist' : 'header.whitelist', {})}
      checked={enabled}
      disabled={blocker !== null || toggling}
      onChange={(e) => toggle(e.currentTarget.checked)}
    />
  );

  return (
    <Card className='flex flex-row! flex-wrap items-center gap-3'>
      <Badge variant='light' color={bedrock ? 'teal' : 'orange'}>
        {tExt(`editions.${edition}`, {})}
      </Badge>
      <Badge variant='dot' color={STATE_COLORS[overview.state]}>
        {tExt(`states.${overview.state}`, {})}
      </Badge>
      {overview.online_mode !== null && (
        <Tooltip label={tExt(overview.online_mode ? 'header.onlineModeHint' : 'header.offlineModeHint', {})}>
          <Badge
            variant='light'
            color={overview.online_mode ? 'blue' : 'yellow'}
            leftSection={<FontAwesomeIcon icon={overview.online_mode ? faShieldHalved : faTriangleExclamation} />}
          >
            {tExt(overview.online_mode ? 'header.onlineMode' : 'header.offlineMode', {})}
          </Badge>
        </Tooltip>
      )}

      <span className='flex-1' />

      {blocker ? (
        <Tooltip label={blocker} multiline maw={260}>
          {whitelistSwitch}
        </Tooltip>
      ) : (
        whitelistSwitch
      )}
      <Tooltip label={tExt('common.refresh', {})}>
        <ActionIcon
          variant='subtle'
          color='gray'
          loading={refreshing}
          aria-label={tExt('common.refresh', {})}
          onClick={onRefresh}
        >
          <FontAwesomeIcon icon={faArrowRotateRight} />
        </ActionIcon>
      </Tooltip>
    </Card>
  );
}
