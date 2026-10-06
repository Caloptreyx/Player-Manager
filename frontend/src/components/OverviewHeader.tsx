import {
  faArrowRotateRight,
  faCube,
  faFileArrowUp,
  faFilePen,
  faShieldHalved,
  faTerminal,
  faTriangleExclamation,
} from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useState } from 'react';
import ActionIcon from '@/elements/buttons/ActionIcon.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Card from '@/elements/data-display/Card.tsx';
import ThemeIcon from '@/elements/data-display/ThemeIcon.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import Switch from '@/elements/input/Switch.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import { setWhitelistEnabled } from '../api.ts';
import { type Access, editHint } from '../lib/access.ts';
import type { ServerState } from '../lib/model.ts';
import { usePlayerManager, useText } from './playerManager.ts';

const STATE_COLORS: Record<ServerState, string> = {
  offline: 'red',
  starting: 'yellow',
  stopping: 'yellow',
  running: 'green',
};

const METHOD_ICONS = { command: faTerminal, file: faFilePen, fileReload: faFileArrowUp };

// game, server state, online mode and how changes are applied on the left; the whitelist switch and refresh on
// the right; a warning underneath only while list changes are blocked
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
  const { state } = overview;
  const { online_mode: onlineMode, whitelist_enabled: whitelistEnabled } = overview.info;
  const hint = editHint(game.edit);

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
  const enabled = whitelistEnabled ?? false;
  const whitelistSwitch = (
    <Switch
      size='md'
      labelPosition='left'
      label={
        <span className='flex items-baseline gap-1.5'>
          <span className='font-medium'>{text('lists.whitelist.title', {})}</span>
          <span className='text-xs text-(--mantine-color-dimmed)'>
            {text(enabled ? 'header.on' : 'header.off', {})}
          </span>
        </span>
      }
      checked={enabled}
      disabled={blocker !== null || toggling}
      onChange={(e) => toggle(e.currentTarget.checked)}
    />
  );

  return (
    <div className='flex flex-col gap-3'>
      <Card className='flex flex-row! flex-wrap items-center gap-x-4 gap-y-3'>
        <div className='flex min-w-0 flex-[1_1_18rem] items-center gap-3'>
          <ThemeIcon size={42} radius='md' variant='light' color={ui.color}>
            <FontAwesomeIcon icon={faCube} size='lg' />
          </ThemeIcon>
          <div className='flex min-w-0 flex-col gap-1'>
            <span className='truncate text-lg leading-tight font-semibold'>{ui.name}</span>
            <div className='flex flex-wrap items-center gap-x-3 gap-y-1.5 text-sm'>
              <span className='inline-flex items-center gap-1.5'>
                <span
                  className='h-2 w-2 rounded-full'
                  style={{ background: `var(--mantine-color-${STATE_COLORS[state]}-filled)` }}
                />
                {text(`states.${state}`, {})}
              </span>
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
              {(hint === 'command' || hint === 'file' || hint === 'fileReload') && (
                <Tooltip label={text(`method.${hint}Hint`, {})} multiline maw={280}>
                  <Badge
                    variant='default'
                    color='gray'
                    className='cursor-help'
                    leftSection={<FontAwesomeIcon icon={METHOD_ICONS[hint]} />}
                  >
                    {text(`method.${hint}`, {})}
                  </Badge>
                </Tooltip>
              )}
            </div>
          </div>
        </div>

        <div className='ml-auto flex items-center gap-3'>
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
              size='lg'
              variant='subtle'
              color='gray'
              loading={refreshing}
              aria-label={text('common.refresh', {})}
              onClick={onRefresh}
            >
              <FontAwesomeIcon icon={faArrowRotateRight} />
            </ActionIcon>
          </Tooltip>
        </div>
      </Card>

      {(hint === 'transition' || hint === 'notRunning') && (
        <Alert color='yellow' icon={<FontAwesomeIcon icon={faTriangleExclamation} />} className='text-sm!'>
          <span className='text-sm'>
            {hint === 'transition'
              ? text('method.transition', { state: text(`states.${state}`, {}).toLowerCase() })
              : text('method.notRunning', {})}
          </span>
        </Alert>
      )}
    </div>
  );
}
