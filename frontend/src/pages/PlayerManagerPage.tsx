import { faCircleInfo, faCubes, faPlug } from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import Card from '@/elements/data-display/Card.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import Tabs from '@/elements/layout/Tabs.tsx';
import { useServerPermissions } from '@/plugins/usePermissions.ts';
import { useServerStore } from '@/stores/server.ts';
import { getOnlinePlayers, getOverview, playerManagerQueryKey } from '../api.ts';
import ConfirmActionModal from '../components/ConfirmActionModal.tsx';
import FileErrorsAlert from '../components/FileErrorsAlert.tsx';
import ListTab from '../components/ListTab.tsx';
import { LIST_STYLES } from '../components/listStyles.ts';
import OnlineTab from '../components/OnlineTab.tsx';
import OverviewHeader from '../components/OverviewHeader.tsx';
import PageSkeleton from '../components/PageSkeleton.tsx';
import { type ConfirmRequest, type PlayerManager, PlayerManagerContext } from '../components/playerManager.ts';
import StatTiles from '../components/StatTiles.tsx';
import { GAME_NAMES, gameText, gameUi } from '../games/index.ts';
import { capabilityAccess, gamePermissions } from '../lib/access.ts';
import type { OnlinePlayers } from '../lib/model.ts';
import { withoutPlayer } from '../lib/players.ts';
import { useExtTranslations } from '../translations.ts';
import useMutationRunner from './useMutationRunner.ts';

export default function PlayerManagerPage() {
  const { t: tExt } = useExtTranslations();
  const queryClient = useQueryClient();
  const serverUuid = useServerStore((state) => state.server.uuid);
  const liveState = useServerStore((state) => state.state);

  const overviewQuery = useQuery({
    queryKey: [...playerManagerQueryKey(serverUuid), 'overview'],
    queryFn: () => getOverview(serverUuid),
  });
  const { run, syncing, restartRequired, dismissRestart } = useMutationRunner(serverUuid);
  const [tab, setTab] = useState<string | null>(null);
  const [pendingConfirm, setPendingConfirm] = useState<ConfirmRequest | null>(null);

  // the websocket knows about power changes first; the overview (and with it the capabilities) follows
  const previousState = useRef(liveState);
  useEffect(() => {
    if (previousState.current === liveState) return;
    previousState.current = liveState;
    queryClient.invalidateQueries({ queryKey: [...playerManagerQueryKey(serverUuid), 'overview'] });
  }, [liveState, queryClient, serverUuid]);

  const overview = overviewQuery.data;
  const game = overview?.game ?? null;
  const state = overview?.state ?? 'offline';

  // the capabilities name the permissions they need; check them all with one hook call
  const permissions = gamePermissions(game);
  const held = useServerPermissions(permissions);
  const granted = new Set(permissions.filter((_, index) => held[index]));

  const online = capabilityAccess(game?.online ?? null, granted);
  const activeTab = tab ?? (state === 'running' && online.visible ? 'online' : (game?.lists[0]?.kind ?? 'online'));

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

  const onlineKey = [...playerManagerQueryKey(serverUuid), 'online'];
  const onlineQuery = useQuery({
    queryKey: onlineKey,
    queryFn: () => getOnlinePlayers(serverUuid),
    // every fetch runs `list` in the console: only when the tab opens or on refresh, never in the background
    enabled: activeTab === 'online' && online.visible && online.blocker === null,
    staleTime: 0,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    retry: false,
  });

  const removeOnline = (name: string) =>
    queryClient.setQueryData<OnlinePlayers>(onlineKey, (current) => current && withoutPlayer(current, name));

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

          <StatTiles activeTab={activeTab} online={onlineQuery.data} onSelect={setTab} />

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
