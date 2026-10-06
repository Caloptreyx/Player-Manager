import {
  faArrowRotateRight,
  faCircleQuestion,
  faFilePen,
  faHourglassHalf,
  faTerminal,
} from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import classNames from 'classnames';
import { useState } from 'react';
import ActionIcon from '@/elements/buttons/ActionIcon.tsx';
import CopyOnClick from '@/elements/CopyOnClick.tsx';
import Avatar from '@/elements/data-display/Avatar.tsx';
import Badge from '@/elements/data-display/Badge.tsx';
import Card from '@/elements/data-display/Card.tsx';
import Tooltip from '@/elements/overlays/Tooltip.tsx';
import FormattedTimestamp from '@/elements/time/FormattedTimestamp.tsx';
import { usePlayerManager, useText } from '../playerManager.ts';
import { useProfileView } from './profileView.ts';

const MODE_ICONS = { live: faTerminal, file: faFilePen, unknown: faCircleQuestion, transition: faHourglassHalf };

/** The full-body skin render, or the avatar when the game has none or it does not load. */
function Body() {
  const { ui } = usePlayerManager();
  const { profile, label } = useProfileView();
  const [failed, setFailed] = useState(false);
  const body = ui.profile.bodyUrl(profile, 320);

  if (body && !failed) {
    return (
      <img
        src={body}
        alt=''
        draggable={false}
        className='h-full w-auto object-contain drop-shadow-md'
        onError={() => setFailed(true)}
      />
    );
  }
  return <Avatar size={72} radius='md' src={ui.avatarUrl(profile, 144)} name={label} />;
}

// who the player is, whether they are online, their game mode and how changes reach them right now
export default function ProfileHeader({ refreshing, onRefresh }: { refreshing: boolean; onRefresh: () => void }) {
  const text = useText();
  const { overview } = usePlayerManager();
  const { profile, mode } = useProfileView();
  // the mode tells online from offline once a complete online list (or a stopped server) settled it
  const online = mode === 'live' ? true : mode === 'file' ? false : null;
  const modeHint =
    mode === 'transition'
      ? text('profile.mode.transitionHint', { state: text(`states.${overview.state}`, {}).toLowerCase() })
      : text(`profile.mode.${mode}Hint`, {});

  return (
    <Card className='flex flex-row! items-center gap-4'>
      <div className='grid h-36 w-24 shrink-0 place-items-center rounded-md bg-(--mantine-color-default-hover) p-2 sm:h-44 sm:w-32'>
        <Body />
      </div>

      <div className='flex min-w-0 flex-1 flex-col items-start gap-2'>
        <div className='flex w-full items-start gap-2'>
          <div className='flex min-w-0 flex-1 flex-col items-start'>
            <h2
              className={classNames(
                'max-w-full truncate text-xl leading-tight font-semibold sm:text-2xl',
                profile.name === null && 'text-(--mantine-color-dimmed) italic',
              )}
            >
              {profile.name ?? text('common.unknownPlayer', {})}
            </h2>
            <Tooltip label={text('common.copyId', {})} className='max-w-full' innerClassName='max-w-full'>
              <CopyOnClick
                content={profile.id}
                className='block max-w-full truncate text-left font-mono text-xs text-(--mantine-color-dimmed) hover:underline'
              >
                {profile.id}
              </CopyOnClick>
            </Tooltip>
          </div>
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

        <div className='flex flex-wrap items-center gap-1.5'>
          {online !== null && (
            <Badge
              variant='light'
              color={online ? 'green' : 'gray'}
              leftSection={
                <span
                  className='block h-1.5 w-1.5 rounded-full'
                  style={{ background: `var(--mantine-color-${online ? 'green' : 'gray'}-filled)` }}
                />
              }
            >
              {text(online ? 'profile.online' : 'profile.offline', {})}
            </Badge>
          )}
          {profile.gamemode && (
            <Badge variant='light' color='blue'>
              {text(`profile.gamemodes.${profile.gamemode}`, {})}
            </Badge>
          )}
          <Tooltip label={modeHint} multiline maw={300}>
            <Badge
              variant='default'
              color='gray'
              className='cursor-help'
              leftSection={<FontAwesomeIcon icon={MODE_ICONS[mode]} />}
            >
              {text(`profile.mode.${mode}`, {})}
            </Badge>
          </Tooltip>
        </div>

        <span className='text-xs text-(--mantine-color-dimmed)'>
          {text('profile.lastSaved', {})}: <FormattedTimestamp timestamp={profile.last_saved} />
          {profile.data_version !== null && (
            <span className='hidden sm:inline'>
              {' '}
              · {text('profile.dataVersion', { version: profile.data_version })}
            </span>
          )}
        </span>
      </div>
    </Card>
  );
}
