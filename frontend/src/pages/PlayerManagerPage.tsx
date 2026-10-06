import { faCircleInfo, faCubes, faPlug, faUsers } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import { useSearchParams } from 'react-router';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import Card from '@/elements/data-display/Card.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import Tabs from '@/elements/layout/Tabs.tsx';
import ConfirmActionModal from '../components/ConfirmActionModal.tsx';
import FileErrorsAlert from '../components/FileErrorsAlert.tsx';
import ListTab from '../components/ListTab.tsx';
import { LIST_STYLES } from '../components/listStyles.ts';
import OnlineTab from '../components/OnlineTab.tsx';
import OverviewHeader from '../components/OverviewHeader.tsx';
import PageSkeleton from '../components/PageSkeleton.tsx';
import PlayersTab from '../components/PlayersTab.tsx';
import { type ConfirmRequest, type PlayerManager, PlayerManagerContext } from '../components/playerManager.ts';
import StatTiles from '../components/StatTiles.tsx';
import { GAME_NAMES, gameText, gameUi } from '../games/index.ts';
import { capabilityAccess } from '../lib/access.ts';
import type { OnlinePlayers } from '../lib/model.ts';
import { withoutPlayer } from '../lib/players.ts';
import { useExtTranslations } from '../translations.ts';
import { onlineKey, overviewKey, useOnlinePlayers, useOverview, useProfiles } from './queries.ts';
import useMutationRunner from './useMutationRunner.tsx';

export default function PlayerManagerPage() {
  const { t: tExt } = useExtTranslations();
  const queryClient = useQueryClient();
  const { serverUuid, overviewQuery, granted } = useOverview();
  const { run, syncing, restartRequired, dismissRestart } = useMutationRunner(() =>
    queryClient.invalidateQueries({ queryKey: overviewKey(serverUuid) }),
  );
  // the tab lives in the url, so coming back from a profile opens the tab it was opened from
  const [searchParams, setSearchParams] = useSearchParams();
  const [pendingConfirm, setPendingConfirm] = useState<ConfirmRequest | null>(null);

  const overview = overviewQuery.data;
  const game = overview?.game ?? null;
  const state = overview?.state ?? 'offline';

  const online = capabilityAccess(game?.online ?? null, granted);
  const profilesView = capabilityAccess(game?.profiles?.view ?? null, granted);
  const tabs = [
    ...(game?.online ? ['online'] : []),
    ...(profilesView.visible ? ['players'] : []),
    ...(game?.lists.map((spec) => spec.kind) ?? []),
  ];
  const requested = searchParams.get('tab');
  const activeTab =
    requested !== null && tabs.includes(requested)
      ? requested
      : state === 'running' && online.visible
        ? 'online'
        : (game?.lists[0]?.kind ?? 'online');
  const setTab = (value: string | null) => {
    if (value === null) return;
    setSearchParams(
      (params) => {
        params.set('tab', value);
        return params;
      },
      { replace: true },
    );
  };

  // the tab row scrolls sideways on narrow screens; keep the active tab (e.g. opened from a tile) in sight
  const tabList = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const list = tabList.current;
    const active = list?.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]');
    if (!list || !active) return;
    const left = active.offsetLeft - list.offsetLeft;
    if (left < list.scrollLeft || left + active.offsetWidth > list.scrollLeft + list.clientWidth) {
      list.scrollTo({ left: left - (list.clientWidth - active.offsetWidth) / 2, behavior: 'smooth' });
    }
  }, [activeTab]);

  // fetched for the online tab and for the online badges of the players tab
  const onlineQuery = useOnlinePlayers(
    serverUuid,
    (activeTab === 'online' || activeTab === 'players') && online.visible && online.blocker === null,
  );
  // the list of saved players also tells which rows get a "View profile" action
  const profilesQuery = useProfiles(serverUuid, profilesView.visible && profilesView.blocker === null);

  const removeOnline = (name: string) =>
    queryClient.setQueryData<OnlinePlayers>(
      onlineKey(serverUuid),
      (current) => current && withoutPlayer(current, name),
    );

  const runConfirmed = async (request: ConfirmRequest, reason: string) => {
    const ok = await run(() => request.run(reason), request.success);
    if (ok) request.onSuccess?.();
    return ok;
  };

  const title = tExt('common.playerManager', {});
  const subtitle = tExt('common.subtitle', {});

  if (!overview) {
    return (
      <ServerContentContainer title={title} subtitle={subtitle}>
        {overviewQuery.isError ? (
          <Alert color='red' title={tExt('errors.loadTitle', {})}>
            <div className='flex flex-col items-start gap-2'>
              <span>{httpErrorToHuman(overviewQuery.error)}</span>
              <Button
                size='xs'
                variant='default'
                onClick={() => {
                  overviewQuery.refetch();
                }}
              >
                {tExt('common.refresh', {})}
              </Button>
            </div>
          </Alert>
        ) : (
          <PageSkeleton />
        )}
      </ServerContentContainer>
    );
  }

  if (!game) {
    return (
      <ServerContentContainer title={title} subtitle={subtitle}>
        <EmptyState
          icon={faCubes}
          title={tExt('notDetected.title', {})}
          description={tExt('notDetected.description', { games: GAME_NAMES.join(', ') })}
        />
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
    profiles: profilesQuery.data ?? null,
    run,
    confirm: setPendingConfirm,
  };

  const notes = ui.notes(tExt);

  return (
    <ServerContentContainer title={title} subtitle={subtitle}>
      <PlayerManagerContext.Provider value={context}>
        <ConfirmActionModal request={pendingConfirm} run={runConfirmed} onClose={() => setPendingConfirm(null)} />

        <div className='flex flex-col gap-4'>
          <OverviewHeader
            toggleAccess={capabilityAccess(game.whitelist_toggle, granted)}
            refreshing={overviewQuery.isFetching || syncing}
            onRefresh={() => overviewQuery.refetch()}
          />

          {restartRequired && (
            <Alert color='yellow' withCloseButton onClose={dismissRestart} className='text-sm!'>
              <span className='text-sm'>
                {text('header.restartRequired', { list: text('lists.whitelist.title', {}) })}
              </span>
            </Alert>
          )}
          <FileErrorsAlert errors={overview.errors} />

          <StatTiles
            activeTab={activeTab}
            online={onlineQuery.data}
            withPlayers={profilesView.visible}
            onSelect={setTab}
          />

          <Card>
            <Tabs
              value={activeTab}
              onChange={setTab}
              keepMounted={false}
              // one row that scrolls sideways on narrow screens instead of wrapping; the list's own bottom line
              // only spans the visible width, so a background that scrolls along draws it under every tab
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
              <Tabs.List ref={tabList}>
                {game.online && (
                  <Tabs.Tab value='online' leftSection={<FontAwesomeIcon icon={faPlug} />}>
                    {text('online.tab', {})}
                  </Tabs.Tab>
                )}
                {profilesView.visible && (
                  <Tabs.Tab value='players' leftSection={<FontAwesomeIcon icon={faUsers} />}>
                    {text('players.tab', {})}
                  </Tabs.Tab>
                )}
                {game.lists.map(({ kind }) => (
                  <Tabs.Tab key={kind} value={kind} leftSection={<FontAwesomeIcon icon={LIST_STYLES[kind].tab} />}>
                    {text(`lists.${kind}.title`, {})}
                  </Tabs.Tab>
                ))}
              </Tabs.List>

              {game.online && (
                <Tabs.Panel value='online' pt='md'>
                  <OnlineTab access={online} query={onlineQuery} onKicked={removeOnline} />
                </Tabs.Panel>
              )}
              {profilesView.visible && (
                <Tabs.Panel value='players' pt='md'>
                  <PlayersTab query={profilesQuery} online={onlineQuery.data} />
                </Tabs.Panel>
              )}
              {game.lists.map((spec) => (
                <Tabs.Panel key={spec.kind} value={spec.kind} pt='md'>
                  <ListTab spec={spec} />
                </Tabs.Panel>
              ))}
            </Tabs>

            {notes.map((note) => (
              <p
                key={note}
                className='mt-4 flex items-start gap-2 border-t border-(--mantine-color-default-border) pt-3 text-xs text-(--mantine-color-dimmed)'
              >
                <FontAwesomeIcon icon={faCircleInfo} className='mt-0.5' />
                {note}
              </p>
            ))}
          </Card>
        </div>
      </PlayerManagerContext.Provider>
    </ServerContentContainer>
  );
}
