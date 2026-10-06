import { useEffect, useRef, useState } from 'react';
import { httpErrorToHuman } from '@/api/axios.ts';
import { useToast } from '@/providers/ToastProvider.tsx';
import type { MutationResult } from '../lib/model.ts';
import { refetchDelay } from '../lib/players.ts';

// runs mutations with toasts (with the server's reply to an RCON command under the message); console commands
// make the server write its files a moment later, so their `refetch` waits a bit while `syncing` is set
export default function useMutationRunner(refetch: () => Promise<unknown>) {
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

  const run = async (action: () => Promise<MutationResult>, success: string): Promise<boolean> => {
    let result: MutationResult;
    try {
      result = await action();
    } catch (error) {
      addToast(httpErrorToHuman(error), 'error');
      return false;
    }

    const reply = result.message?.trim();
    addToast(
      reply ? (
        <span className='flex flex-col gap-0.5'>
          <span>{success}</span>
          <span className='font-mono text-xs break-words opacity-75'>{reply}</span>
        </span>
      ) : (
        success
      ),
      'success',
    );
    if (result.restart_required) setRestartRequired(true);

    const delay = refetchDelay(result.method);
    if (delay === 0) {
      await refetch();
    } else {
      setSyncing((count) => count + 1);
      const timer = window.setTimeout(() => {
        timers.current.delete(timer);
        refetch().finally(() => setSyncing((count) => count - 1));
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
