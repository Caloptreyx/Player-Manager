import {
  faBan,
  faListCheck,
  faNetworkWired,
  faTrash,
  faUnlock,
  faUserPlus,
  faUserShield,
  type IconDefinition,
} from '@fortawesome/free-solid-svg-icons';
import type { ListKind } from '../lib/model.ts';

export interface ListStyle {
  tab: IconDefinition;
  add: IconDefinition;
  remove: IconDefinition;
  /** Accent of the stat tile. */
  color: string;
  /** Adding keeps players out: red submit button. */
  danger: boolean;
}

export const LIST_STYLES: Record<ListKind, ListStyle> = {
  whitelist: { tab: faListCheck, add: faUserPlus, remove: faTrash, color: 'blue', danger: false },
  operators: { tab: faUserShield, add: faUserShield, remove: faTrash, color: 'violet', danger: false },
  bans: { tab: faBan, add: faBan, remove: faUnlock, color: 'red', danger: true },
  ip_bans: { tab: faNetworkWired, add: faBan, remove: faUnlock, color: 'red', danger: true },
};
