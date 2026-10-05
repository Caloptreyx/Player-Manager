import { createContext, useContext } from 'react';
import { type GameText, type GameUi, gameText } from '../games/index.ts';
import type { Access } from '../lib/access.ts';
import type { Game, MutationResult, Overview } from '../lib/model.ts';
import { useExtTranslations } from '../translations.ts';

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
  game: Game;
  ui: GameUi;
  overview: Overview;
  /** Adding to and removing from the lists. */
  editAccess: Access;
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

export function useText(): GameText {
  const { t } = useExtTranslations();
  return gameText(t, usePlayerManager().ui);
}
