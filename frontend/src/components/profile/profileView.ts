import { createContext, useContext } from 'react';
import type { Access } from '../../lib/access.ts';
import type { Profile, ProfileAction } from '../../lib/model.ts';
import type { ProfileMode } from '../../lib/profiles.ts';

export interface ProfileView {
  profile: Profile;
  /** Name, or the id of a player without a known name. */
  label: string;
  mode: ProfileMode;
  /** Clearing slots and containers, game mode and level. */
  editAccess: Access;
  /** Giving items, which only works as a command. */
  giveAccess: Access;
  /** Runs a profile action with toasts and refetches the profile; resolves to whether it succeeded. */
  act: (body: ProfileAction, success: string) => Promise<boolean>;
}

export const ProfileViewContext = createContext<ProfileView | null>(null);

export function useProfileView(): ProfileView {
  const context = useContext(ProfileViewContext);
  if (!context) throw new Error('useProfileView must be used inside the profile page');

  return context;
}
