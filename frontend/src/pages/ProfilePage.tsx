import {
  faArrowLeft,
  faBoxArchive,
  faChartColumn,
  faCubes,
  faGaugeHigh,
  faShirt,
  faTrophy,
  faUserSlash,
} from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { Skeleton } from '@mantine/core';
import { useQueryClient } from '@tanstack/react-query';
import { useState } from 'react';
import { Link, useParams } from 'react-router';
import { getHttpStatus, httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import Card from '@/elements/data-display/Card.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import Tabs from '@/elements/layout/Tabs.tsx';
import { runProfileAction } from '../api.ts';
import ConfirmActionModal from '../components/ConfirmActionModal.tsx';
import { type ConfirmRequest, type PlayerManager, PlayerManagerContext } from '../components/playerManager.ts';
import AdvancementsSection from '../components/profile/AdvancementsSection.tsx';
import { EnderChestSection, InventorySection } from '../components/profile/ContainerSections.tsx';
import OverviewSection from '../components/profile/OverviewSection.tsx';
import ProfileHeader from '../components/profile/ProfileHeader.tsx';
import { type ProfileView, ProfileViewContext } from '../components/profile/profileView.ts';
import StatsSection from '../components/profile/StatsSection.tsx';
import { gameText, gameUi } from '../games/index.ts';
import { capabilityAccess } from '../lib/access.ts';
import { profileActionAccess, profileMode } from '../lib/profiles.ts';
import { useExtTranslations } from '../translations.ts';
import { profilesKey, useOnlinePlayers, useOverview, useProfile } from './queries.ts';
import useMutationRunner from './useMutationRunner.tsx';

const TABS = [
  { value: 'overview', icon: faGaugeHigh, label: 'profile.tabs.overview' },
  { value: 'inventory', icon: faShirt, label: 'profile.tabs.inventory' },
  { value: 'enderChest', icon: faBoxArchive, label: 'profile.tabs.enderChest' },
  { value: 'stats', icon: faChartColumn, label: 'profile.tabs.stats' },
  { value: 'advancements', icon: faTrophy, label: 'profile.tabs.advancements' },
] as const;

function ProfileSkeleton() {
  return (
    <div className='flex flex-col gap-4'>
      <Card className='flex flex-row! items-center gap-4'>
        <Skeleton height={176} width={128} radius='md' />
        <div className='flex flex-1 flex-col gap-3'>
          <Skeleton height={24} width='40%' />
          <Skeleton height={12} width='60%' />
          <Skeleton height={20} width='50%' />
        </div>
      </Card>
      <Card>
        <Skeleton height={32} width='70%' mb='md' />
        <div className='grid grid-cols-1 gap-3 md:grid-cols-2'>
          {Array.from({ length: 4 }, (_, index) => (
            <Skeleton key={index} height={120} radius='md' />
          ))}
        </div>
      </Card>
    </div>
  );
}

// one player's saved data (vitals, inventory, ender chest, stats, advancements) as a sub-page of the player
// manager; actions run as commands while the player is online and as file edits otherwise
export default function ProfilePage() {
  const { t: tExt } = useExtTranslations();
  const queryClient = useQueryClient();
  const { profileId = '' } = useParams<'profileId'>();
  const { serverUuid, overviewQuery, granted } = useOverview();
  const [tab, setTab] = useState<string | null>('overview');
  const [pendingConfirm, setPendingConfirm] = useState<ConfirmRequest | null>(null);

  const overview = overviewQuery.data;
  const game = overview?.game ?? null;
  const state = overview?.state ?? 'offline';
  const view = capabilityAccess(game?.profiles?.view ?? null, granted);
  const online = capabilityAccess(game?.online ?? null, granted);

  // whether the player is online decides how actions run; the list refreshes itself where that is silent
  const onlineUsable = state === 'running' && online.visible && online.blocker === null;
  const onlineQuery = useOnlinePlayers(serverUuid, onlineUsable);
  const profileQuery = useProfile(serverUuid, profileId, view.visible && view.blocker === null);
  const refetch = () => queryClient.invalidateQueries({ queryKey: profilesKey(serverUuid) });
  const { run, syncing } = useMutationRunner(refetch);

  const runConfirmed = async (request: ConfirmRequest, reason: string) => {
    const ok = await run(() => request.run(reason), request.success);
    if (ok) request.onSuccess?.();
    return ok;
  };

  const title = tExt('profile.title', {});
  const back = (
    <Link
      to={{ pathname: '..', search: '?tab=players' }}
      relative='path'
      className='inline-flex w-fit items-center gap-2 text-sm text-(--mantine-color-dimmed) hover:text-(--mantine-color-text)'
    >
      <FontAwesomeIcon icon={faArrowLeft} />
      {tExt('profile.back', {})}
    </Link>
  );

  const profile = profileQuery.data;
  if (!overview || profileQuery.isLoading) {
    const failed = overviewQuery.isError ? overviewQuery.error : null;
    return (
      <ServerContentContainer title={title} hideTitleComponent>
        <div className='flex flex-col gap-4'>
          {back}
          {failed ? (
            <Alert color='red' title={tExt('errors.loadTitle', {})}>
              {httpErrorToHuman(failed)}
            </Alert>
          ) : (
            <ProfileSkeleton />
          )}
        </div>
      </ServerContentContainer>
    );
  }

  if (!game?.profiles || !view.visible || !profile) {
    const notFound = profileQuery.isError && getHttpStatus(profileQuery.error) === 404;
    return (
      <ServerContentContainer title={title} hideTitleComponent>
        <div className='flex flex-col gap-4'>
          {back}
          {profileQuery.isError && !notFound ? (
            <Alert color='red' title={tExt('profile.loadTitle', {})}>
              <div className='flex flex-col items-start gap-2'>
                <span>{httpErrorToHuman(profileQuery.error)}</span>
                <Button
                  size='xs'
                  variant='default'
                  onClick={() => {
                    profileQuery.refetch();
                  }}
                >
                  {tExt('common.retry', {})}
                </Button>
              </div>
            </Alert>
          ) : (
            <Card>
              <EmptyState
                flush
                icon={notFound ? faUserSlash : faCubes}
                title={tExt(notFound ? 'profile.notFoundTitle' : 'notDetected.title', {})}
                description={tExt(notFound ? 'profile.notFound' : 'profile.unsupported', {})}
              />
            </Card>
          )}
        </div>
      </ServerContentContainer>
    );
  }

  const ui = gameUi(game.id);
  const text = gameText(tExt, ui);
  const context: PlayerManager = {
    serverUuid,
    game,
    ui,
    overview,
    editAccess: capabilityAccess(game.edit, granted),
    kickAccess: capabilityAccess(game.kick, granted),
    profiles: null,
    run,
    confirm: setPendingConfirm,
  };

  const mode = profileMode(state, onlineQuery.data, profile);
  const live = capabilityAccess(game.profiles.edit_live, granted);
  const offline = capabilityAccess(game.profiles.edit_offline, granted);
  const profileView: ProfileView = {
    profile,
    label: profile.name ?? profile.id,
    mode,
    editAccess: profileActionAccess(mode, live, offline, false),
    giveAccess: profileActionAccess(mode, live, offline, true),
    act: (body, success) => run(() => runProfileAction(serverUuid, profile.id, body), success),
  };

  return (
    <ServerContentContainer title={profile.name ?? title} hideTitleComponent>
      <PlayerManagerContext.Provider value={context}>
        <ProfileViewContext.Provider value={profileView}>
          <ConfirmActionModal request={pendingConfirm} run={runConfirmed} onClose={() => setPendingConfirm(null)} />

          <div className='flex flex-col gap-4'>
            {back}
            <ProfileHeader
              refreshing={profileQuery.isFetching || onlineQuery.isFetching || syncing}
              onRefresh={() => {
                profileQuery.refetch();
                if (onlineUsable) onlineQuery.refetch();
              }}
            />

            <Card>
              <Tabs
                value={tab}
                onChange={setTab}
                keepMounted={false}
                classNames={{
                  list: 'overflow-x-auto overflow-y-hidden [scrollbar-width:none] [&::-webkit-scrollbar]:hidden',
                  tab: 'shrink-0',
                }}
                styles={{
                  list: {
                    flexWrap: 'nowrap',
                    background:
                      'linear-gradient(var(--mantine-color-default-border), var(--mantine-color-default-border)) left bottom / 100% 2px no-repeat local',
                  },
                }}
              >
                <Tabs.List>
                  {TABS.map(({ value, icon, label }) => (
                    <Tabs.Tab key={value} value={value} leftSection={<FontAwesomeIcon icon={icon} />}>
                      {text(label, {})}
                    </Tabs.Tab>
                  ))}
                </Tabs.List>
                <Tabs.Panel value='overview' pt='md'>
                  <OverviewSection />
                </Tabs.Panel>
                <Tabs.Panel value='inventory' pt='md'>
                  <InventorySection />
                </Tabs.Panel>
                <Tabs.Panel value='enderChest' pt='md'>
                  <EnderChestSection />
                </Tabs.Panel>
                <Tabs.Panel value='stats' pt='md'>
                  <StatsSection />
                </Tabs.Panel>
                <Tabs.Panel value='advancements' pt='md'>
                  <AdvancementsSection />
                </Tabs.Panel>
              </Tabs>
            </Card>
          </div>
        </ProfileViewContext.Provider>
      </PlayerManagerContext.Provider>
    </ServerContentContainer>
  );
}
