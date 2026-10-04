import {
  faBan,
  faCircleInfo,
  faCubes,
  faListCheck,
  faNetworkWired,
  faPlug,
  faUserShield,
} from '@fortawesome/free-solid-svg-icons';
import { FontAwesomeIcon } from '@fortawesome/react-fontawesome';
import { useQuery, useQueryClient } from '@tanstack/react-query';
import { type ReactNode, useEffect, useRef, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import Button from '@/elements/buttons/Button.tsx';
import ServerContentContainer from '@/elements/containers/ServerContentContainer.tsx';
import Card from '@/elements/data-display/Card.tsx';
import Alert from '@/elements/feedback/Alert.tsx';
import EmptyState from '@/elements/feedback/EmptyState.tsx';
import Spinner from '@/elements/feedback/Spinner.tsx';
import Tabs from '@/elements/layout/Tabs.tsx';
import { useServerCan } from '@/plugins/usePermissions.ts';
import { useServerStore } from '@/stores/server.ts';
import { getOnlinePlayers, getOverview, playerManagerQueryKey } from '../api.ts';
import BansTab from '../components/BansTab.tsx';
import ConfirmActionModal from '../components/ConfirmActionModal.tsx';
import FileErrorsAlert from '../components/FileErrorsAlert.tsx';
import IpBansTab from '../components/IpBansTab.tsx';
import OnlineTab from '../components/OnlineTab.tsx';
import OperatorsTab from '../components/OperatorsTab.tsx';
import OverviewHeader from '../components/OverviewHeader.tsx';
import { type ConfirmRequest, type PlayerManager, PlayerManagerContext } from '../components/playerManager.ts';
import WhitelistTab from '../components/WhitelistTab.tsx';
import {
  type Access,
  type Granted,
  kickAccess,
  listAccess,
  mutationMethod,
  onlineAccess,
  whitelistToggleAccess,
} from '../lib/access.ts';
import { type OnlinePlayers, withoutPlayer } from '../lib/players.ts';
import { useExtTranslations } from '../translations.ts';
import useMutationRunner from './useMutationRunner.ts';

const NO_ACCESS: Access = { visible: false, blocker: null };

function TabCount({ children }: { children: ReactNode }) {
  return (
    <span className='rounded-full bg-(--mantine-color-default-hover) px-1.5 py-px text-[10px] leading-4'>
      {children}
    </span>
  );
}

export default function PlayerManagerPage() {
  const { t: tExt } = useExtTranslations();
  const queryClient = useQueryClient();
  const serverUuid = useServerStore((state) => state.server.uuid);
  const liveState = useServerStore((state) => state.state);

  const granted: Granted = {
    console: useServerCan('control.console'),
    readConsole: useServerCan('control.read-console'),
    writeFiles: useServerCan('files.create'),
  };

  const overviewQuery = useQuery({
    queryKey: [...playerManagerQueryKey(serverUuid), 'overview'],
    queryFn: () => getOverview(serverUuid),
  });
  const { run, syncing, restartRequired, dismissRestart } = useMutationRunner(serverUuid);
  const [tab, setTab] = useState<string | null>(null);
  const [pendingConfirm, setPendingConfirm] = useState<ConfirmRequest | null>(null);

  // the websocket knows about power changes first; the overview (and with it the method) follows
  const previousState = useRef(liveState);
  useEffect(() => {
    if (previousState.current === liveState) return;
    previousState.current = liveState;
    queryClient.invalidateQueries({ queryKey: [...playerManagerQueryKey(serverUuid), 'overview'] });
  }, [liveState, queryClient, serverUuid]);

  const overview = overviewQuery.data;
  const edition = overview?.edition ?? null;
  const state = overview?.state ?? 'offline';
  const online = edition ? onlineAccess(state, granted) : NO_ACCESS;
  const activeTab = tab ?? (state === 'running' && online.visible ? 'online' : 'whitelist');

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
          <Spinner.Centered />
        )}
      </ServerContentContainer>
    );
  }

  if (!edition) {
    return (
      <ServerContentContainer title={title} subtitle={subtitle}>
        <EmptyState
          icon={faCubes}
          title={tExt('notDetected.title', {})}
          description={tExt('notDetected.description', {})}
        />
      </ServerContentContainer>
    );
  }

  const bedrock = edition === 'bedrock';
  const method = mutationMethod(edition, state);
  const context: PlayerManager = {
    serverUuid,
    edition,
    overview,
    method,
    listAccess: listAccess(edition, state, granted),
    kickAccess: kickAccess(state, granted),
    run,
    confirm: setPendingConfirm,
  };

  const methodText =
    method === null
      ? tExt('method.transition', { state: tExt(`states.${state}`, {}).toLowerCase() })
      : bedrock
        ? tExt(state === 'running' ? 'method.bedrockRunning' : 'method.bedrockOffline', {})
        : tExt(state === 'running' ? 'method.javaRunning' : 'method.javaOffline', {});

  return (
    <ServerContentContainer title={title} subtitle={subtitle}>
      <PlayerManagerContext.Provider value={context}>
        <ConfirmActionModal request={pendingConfirm} run={runConfirmed} onClose={() => setPendingConfirm(null)} />

        <div className='flex flex-col gap-4'>
          <OverviewHeader
            toggleAccess={whitelistToggleAccess(edition, state, granted)}
            refreshing={overviewQuery.isFetching || syncing}
            onRefresh={() => overviewQuery.refetch()}
          />

          {restartRequired && (
            <Alert color='yellow' withCloseButton onClose={dismissRestart}>
              {tExt('header.restartRequired', { list: tExt(bedrock ? 'header.allowlist' : 'header.whitelist', {}) })}
            </Alert>
          )}
          <FileErrorsAlert errors={overview.errors} />

          <p className='flex items-center gap-2 text-sm text-(--mantine-color-dimmed)'>
            <FontAwesomeIcon icon={faCircleInfo} />
            {methodText}
          </p>

          <Card>
            <Tabs value={activeTab} onChange={setTab} keepMounted={false}>
              <Tabs.List>
                <Tabs.Tab
                  value='online'
                  leftSection={<FontAwesomeIcon icon={faPlug} />}
                  rightSection={
                    onlineQuery.data && (
                      <TabCount>
                        {onlineQuery.data.count}/{onlineQuery.data.max}
                      </TabCount>
                    )
                  }
                >
                  {tExt('tabs.online', {})}
                </Tabs.Tab>
                <Tabs.Tab
                  value='whitelist'
                  leftSection={<FontAwesomeIcon icon={faListCheck} />}
                  rightSection={<TabCount>{overview.whitelist.length}</TabCount>}
                >
                  {tExt(bedrock ? 'tabs.allowlist' : 'tabs.whitelist', {})}
                </Tabs.Tab>
                <Tabs.Tab
                  value='operators'
                  leftSection={<FontAwesomeIcon icon={faUserShield} />}
                  rightSection={<TabCount>{overview.operators.length}</TabCount>}
                >
                  {tExt('tabs.operators', {})}
                </Tabs.Tab>
                <Tabs.Tab
                  value='bans'
                  leftSection={<FontAwesomeIcon icon={faBan} />}
                  rightSection={!bedrock && <TabCount>{overview.bans.length}</TabCount>}
                >
                  {tExt('tabs.bans', {})}
                </Tabs.Tab>
                {!bedrock && (
                  <Tabs.Tab
                    value='ip-bans'
                    leftSection={<FontAwesomeIcon icon={faNetworkWired} />}
                    rightSection={<TabCount>{overview.ip_bans.length}</TabCount>}
                  >
                    {tExt('tabs.ipBans', {})}
                  </Tabs.Tab>
                )}
              </Tabs.List>

              <Tabs.Panel value='online' pt='md'>
                <OnlineTab access={online} query={onlineQuery} onKicked={removeOnline} />
              </Tabs.Panel>
              <Tabs.Panel value='whitelist' pt='md'>
                <WhitelistTab />
              </Tabs.Panel>
              <Tabs.Panel value='operators' pt='md'>
                <OperatorsTab />
              </Tabs.Panel>
              <Tabs.Panel value='bans' pt='md'>
                {bedrock ? (
                  <EmptyState
                    flush
                    icon={faBan}
                    title={tExt('bedrockBans.title', {})}
                    description={tExt('bedrockBans.description', {})}
                  />
                ) : (
                  <BansTab />
                )}
              </Tabs.Panel>
              {!bedrock && (
                <Tabs.Panel value='ip-bans' pt='md'>
                  <IpBansTab />
                </Tabs.Panel>
              )}
            </Tabs>
          </Card>
        </div>
      </PlayerManagerContext.Provider>
    </ServerContentContainer>
  );
}
