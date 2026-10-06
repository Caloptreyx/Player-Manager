import type { Game } from './model.ts';

// client-side mirror of the backend validation, for form feedback only; the backend stays authoritative and
// checks names and ids against the same patterns the game descriptor carries
export type FieldError = 'required' | 'name' | 'id' | 'reasonLength' | 'reasonControl' | 'ip' | 'duplicate' | 'itemId';

export const REASON_MAX_LENGTH = 256;

/** Bounds of the profile actions, as the backend checks them. */
export const XP_LEVEL_MAX = 21863;
export const GIVE_COUNT_MAX = 6400;

const ITEM_ID = /^[a-z0-9_.-]+:[a-z0-9_./-]+$/;

/** The item id a give sends: trimmed, `minecraft:` when the namespace is left out; null when invalid. */
export const normalizeItemId = (input: string): string | null => {
  const trimmed = input.trim();
  const id = trimmed.includes(':') ? trimmed : `minecraft:${trimmed}`;
  return ITEM_ID.test(id) ? id : null;
};

export const validateItemId = (input: string): FieldError | null => {
  if (input.trim() === '') return 'required';
  return normalizeItemId(input) === null ? 'itemId' : null;
};

export const validateName = (game: Pick<Game, 'player_name'>, name: string): FieldError | null => {
  if (name === '') return 'required';
  return new RegExp(game.player_name.pattern).test(name) ? null : 'name';
};

/** An empty id is valid: ids are optional and looked up from the name. */
export const validateId = (game: Pick<Game, 'player_id'>, id: string): FieldError | null => {
  if (id === '') return null;
  return new RegExp(game.player_id.pattern).test(id) ? null : 'id';
};

export const validateReason = (reason: string): FieldError | null => {
  // the backend counts unicode scalar values, not UTF-16 code units
  if ([...reason].length > REASON_MAX_LENGTH) return 'reasonLength';
  return /\p{Cc}/u.test(reason) ? 'reasonControl' : null;
};

const isIpv4 = (value: string): boolean => {
  const parts = value.split('.');
  return parts.length === 4 && parts.every((part) => /^(?:0|[1-9]\d{0,2})$/.test(part) && Number(part) <= 255);
};

const groupCount = (groups: string[], allowIpv4Tail: boolean): number | null => {
  let count = 0;
  for (const [index, group] of groups.entries()) {
    if (allowIpv4Tail && index === groups.length - 1 && group.includes('.')) {
      if (!isIpv4(group)) return null;
      count += 2;
    } else if (/^[0-9a-f]{1,4}$/i.test(group)) {
      count += 1;
    } else {
      return null;
    }
  }
  return count;
};

const isIpv6 = (value: string): boolean => {
  const halves = value.split('::');
  if (halves.length > 2) return false;
  if (halves.length === 1) return groupCount(value.split(':'), true) === 8;

  const [head, tail] = halves;
  const headCount = head === '' ? 0 : groupCount(head.split(':'), false);
  const tailCount = tail === '' ? 0 : groupCount(tail.split(':'), true);
  // `::` stands for at least one zero group
  return headCount !== null && tailCount !== null && headCount + tailCount <= 7;
};

export const validateIp = (ip: string): FieldError | null => {
  if (ip === '') return 'required';
  return isIpv4(ip) || isIpv6(ip) ? null : 'ip';
};
