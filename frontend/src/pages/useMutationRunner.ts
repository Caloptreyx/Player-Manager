import { useQueryClient } from '@tanstack/react-query';
import { useEffect, useRef, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import { type MutationResult, playerManagerQueryKey } from '../api.ts';
import { refetchDelay } from '../lib/players.ts';

// runs mutations with toasts; console commands make the server write its files a moment later, so their
// overview refetch waits a bit while `syncing` is set
export default function useMutationRunner(serverUuid: string) {
  const queryClient = useQueryClient();
  const { addToast } = useToast();
  const [syncing, setSyncing] = useState(0);
  const [restartRequired, setRestartRequired] = useState(false);
  const timers = useRef(new Set<number>());

  useEffect(() => {
    const pending = timers.current;
    return () => {
      for (const timer of pending) clearTimeout(timer);
      pending.clear();
    };
  }, []);

  const refetchOverview = () =>
    queryClient.invalidateQueries({ queryKey: [...playerManagerQueryKey(serverUuid), 'overview'] });

  const run = async (action: () => Promise<MutationResult>, success: string): Promise<boolean> => {
    let result: MutationResult;
    try {
      result = await action();
    } catch (error) {
      addToast(httpErrorToHuman(error), 'error');
      return false;
    }

    addToast(success, 'success');
    if (result.restart_required) setRestartRequired(true);

    const delay = refetchDelay(result.method);
    if (delay === 0) {
      await refetchOverview();
    } else {
      setSyncing((count) => count + 1);
      const timer = window.setTimeout(() => {
        timers.current.delete(timer);
        refetchOverview().finally(() => setSyncing((count) => count - 1));
      }, delay);
      timers.current.add(timer);
    }
    return true;
  };

  return {
    run,
    syncing: syncing > 0,
    restartRequired,
    dismissRestart: () => setRestartRequired(false),
  };
}
