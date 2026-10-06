import { useQuery, useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef } from 'react';
import { useServerPermissions } from '@/plugins/usePermissions.ts';
import { useServerStore } from '@/stores/server.ts';
import { getOnlinePlayers, getOverview, getProfile, getProfiles, playerManagerQueryKey } from '../api.ts';
import { gamePermissions } from '../lib/access.ts';
import { onlineRefreshInterval } from '../lib/players.ts';

// the queries the player manager page and the profile page share, under one key per server

export const overviewKey = (serverUuid: string) => [...playerManagerQueryKey(serverUuid), 'overview'];
export const onlineKey = (serverUuid: string) => [...playerManagerQueryKey(serverUuid), 'online'];
export const profilesKey = (serverUuid: string) => [...playerManagerQueryKey(serverUuid), 'profiles'];
export const profileKey = (serverUuid: string, id: string) => [...profilesKey(serverUuid), id];

/** The overview of the server, refetched on power changes, plus the panel permissions its capabilities name. */
export function useOverview() {
  const queryClient = useQueryClient();
  const serverUuid = useServerStore((state) => state.server.uuid);
  const liveState = useServerStore((state) => state.state);

  const overviewQuery = useQuery({ queryKey: overviewKey(serverUuid), queryFn: () => getOverview(serverUuid) });

  // the websocket knows about power changes first; the overview (and with it the capabilities) follows
  const previousState = useRef(liveState);
  useEffect(() => {
    if (previousState.current === liveState) return;
    previousState.current = liveState;
    queryClient.invalidateQueries({ queryKey: overviewKey(serverUuid) });
  }, [liveState, queryClient, serverUuid]);

  // the capabilities name the permissions they need; check them all with one hook call
  const permissions = gamePermissions(overviewQuery.data?.game ?? null);
  const held = useServerPermissions(permissions);
  const granted = new Set(permissions.filter((_, index) => held[index]));

  return { serverUuid, overviewQuery, granted };
}

/**
 * Who is online. Fetched when a view needs it and on refresh; it also refreshes itself every few seconds while
 * the page is visible, unless the answer came from the console (each fetch would run `list` there).
 */
export const useOnlinePlayers = (serverUuid: string, enabled: boolean) =>
  useQuery({
    queryKey: onlineKey(serverUuid),
    queryFn: () => getOnlinePlayers(serverUuid),
    enabled,
    staleTime: 0,
    refetchOnWindowFocus: false,
    refetchOnReconnect: false,
    retry: false,
    refetchInterval: (query) => onlineRefreshInterval(query.state.data, query.state.status === 'error'),
    refetchIntervalInBackground: false,
  });

export const useProfiles = (serverUuid: string, enabled: boolean) =>
  useQuery({ queryKey: profilesKey(serverUuid), queryFn: () => getProfiles(serverUuid), enabled });

export const useProfile = (serverUuid: string, id: string, enabled: boolean) =>
  useQuery({ queryKey: profileKey(serverUuid, id), queryFn: () => getProfile(serverUuid, id), enabled, retry: false });
