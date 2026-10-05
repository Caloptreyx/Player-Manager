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
import type { ServerState } from '../lib/model.ts';
import { usePlayerManager, useText } from './playerManager.ts';

const STATE_COLORS: Record<ServerState, string> = {
  offline: 'red',
  starting: 'yellow',
  stopping: 'yellow',
  running: 'green',
};

// game, server state, online mode and the whitelist switch, above the tabs
export default function OverviewHeader({
  toggleAccess,
  refreshing,
  onRefresh,
}: {
  toggleAccess: Access;
  refreshing: boolean;
  onRefresh: () => void;
}) {
  const text = useText();
  const { serverUuid, game, ui, overview, run } = usePlayerManager();
  const [toggling, setToggling] = useState(false);
  const { online_mode: onlineMode, whitelist_enabled: whitelistEnabled } = overview.info;

  const toggle = async (next: boolean) => {
    setToggling(true);
    await run(
      () => setWhitelistEnabled(serverUuid, next),
      text(next ? 'lists.whitelist.on' : 'lists.whitelist.off', {}),
    );
    setToggling(false);
  };

  const blocker = toggleAccess.visible
    ? toggleAccess.blocker && text(`blockers.${toggleAccess.blocker}`, {})
    : text('blockers.noPermission', {});
  const whitelistSwitch = (
    <Switch
      label={text('lists.whitelist.title', {})}
      checked={whitelistEnabled ?? false}
      disabled={blocker !== null || toggling}
      onChange={(e) => toggle(e.currentTarget.checked)}
    />
  );

  return (
    <Card className='flex flex-row! flex-wrap items-center gap-3'>
      <Badge variant='light' color={ui.color}>
        {ui.name}
      </Badge>
      <Badge variant='dot' color={STATE_COLORS[overview.state]}>
        {text(`states.${overview.state}`, {})}
      </Badge>
      {onlineMode !== null && (
        <Tooltip label={text(onlineMode ? 'header.onlineModeHint' : 'header.offlineModeHint', {})}>
          <Badge
            variant='light'
            color={onlineMode ? 'blue' : 'yellow'}
            leftSection={<FontAwesomeIcon icon={onlineMode ? faShieldHalved : faTriangleExclamation} />}
          >
            {text(onlineMode ? 'header.onlineMode' : 'header.offlineMode', {})}
          </Badge>
        </Tooltip>
      )}

      <span className='flex-1' />

      {game.whitelist_toggle &&
        (blocker ? (
          <Tooltip label={blocker} multiline maw={260}>
            {whitelistSwitch}
          </Tooltip>
        ) : (
          whitelistSwitch
        ))}
      <Tooltip label={text('common.refresh', {})}>
        <ActionIcon
          variant='subtle'
          color='gray'
          loading={refreshing}
          aria-label={text('common.refresh', {})}
          onClick={onRefresh}
        >
          <FontAwesomeIcon icon={faArrowRotateRight} />
        </ActionIcon>
      </Tooltip>
    </Card>
  );
}
