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
  /** Adding keeps players out: red submit button. */
  danger: boolean;
}

export const LIST_STYLES: Record<ListKind, ListStyle> = {
  whitelist: { tab: faListCheck, add: faUserPlus, remove: faTrash, danger: false },
  operators: { tab: faUserShield, add: faUserShield, remove: faTrash, danger: false },
  bans: { tab: faBan, add: faBan, remove: faUnlock, danger: true },
  ip_bans: { tab: faNetworkWired, add: faBan, remove: faUnlock, danger: true },
};
