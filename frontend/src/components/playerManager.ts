import { createContext, useContext } from 'react';
import type { MutationResult } from '../api.ts';
import type { Access, Method } from '../lib/access.ts';
import type { Edition, Overview } from '../lib/players.ts';

export interface ConfirmRequest {
  title: string;
  content: string;
  confirm: string;
  /** Asks for an optional reason (kick, ban) before running. */
  withReason?: boolean;
  run: (reason: string) => Promise<MutationResult>;
  success: string;
  onSuccess?: () => void;
}

export interface PlayerManager {
  serverUuid: string;
  edition: Edition;
  overview: Overview;
  /** How list changes are carried out right now, null while starting/stopping. */
  method: Method | null;
  listAccess: Access;
  kickAccess: Access;
  /** Runs a mutation, toasts the outcome and schedules the overview refetch; resolves to whether it succeeded. */
  run: (action: () => Promise<MutationResult>, success: string) => Promise<boolean>;
  /** Opens the confirmation dialog of a destructive action. */
  confirm: (request: ConfirmRequest) => void;
}

export const PlayerManagerContext = createContext<PlayerManager | null>(null);

export function usePlayerManager(): PlayerManager {
  const context = useContext(PlayerManagerContext);
  if (!context) throw new Error('usePlayerManager must be used inside the player manager page');

  return context;
}
