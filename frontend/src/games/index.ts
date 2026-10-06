import type { ListSpec } from '../lib/model.ts';
import type { PlayerRef } from '../lib/players.ts';
import type { ExtT, ExtValues, GenericKey, Wording } from '../translations.ts';
import minecraftBedrock from './minecraftBedrock.ts';
import minecraftJava from './minecraftJava.ts';

// the page renders everything from the game descriptor of the overview; these optional modules only add what
// the descriptor cannot carry (names, avatars, wording), so a game without one still works with the defaults

export interface GameUi {
  /** Display name in the header; product names stay untranslated. */
  name: string;
  /** Color of the game icon in the header. */
  color: string;
  /** Avatar image of a player, `size` pixels wide (default 32); null shows the initial of the name. */
  avatarUrl: (player: PlayerRef, size?: number) => string | null;
  /** Generic texts the game words differently. */
  wording: Wording;
  /** Label of an operator level on badges and in the level select. */
  levelLabel: (t: ExtT, level: string, place: 'badge' | 'option') => string;
  /** Levels with operator rights; the others (Bedrock members, visitors) are plain permission entries. */
  isOperatorLevel: (level: string) => boolean;
  /** An extra hint in the add form of a list. */
  addFormNote: (t: ExtT, spec: ListSpec) => string | null;
  /** Informational lines under the lists, e.g. why a list players look for does not exist. */
  notes: (t: ExtT) => string[];
}

/** A per-game module: the name plus whatever differs from the generic UI. */
export type GameModule = Pick<GameUi, 'name'> & Partial<GameUi>;

const GENERIC: Omit<GameUi, 'name'> = {
  color: 'gray',
  avatarUrl: () => null,
  wording: {},
  levelLabel: (_t, level) => level,
  isOperatorLevel: () => true,
  addFormNote: () => null,
  notes: () => [],
};

/** Keyed by `Game.id`. */
const GAMES: Record<string, GameModule> = {
  minecraft_java: minecraftJava,
  minecraft_bedrock: minecraftBedrock,
};

/** The UI module of a game; an unknown game gets the generic UI under its id. */
export const gameUi = (id: string): GameUi => ({
  ...GENERIC,
  name: id,
  ...(Object.hasOwn(GAMES, id) ? GAMES[id] : {}),
});

/** Names of the games with a UI module, for the "nothing detected" page. */
export const GAME_NAMES = Object.values(GAMES).map((game) => game.name);

export type GameText = <K extends GenericKey>(key: K, values: ExtValues<K>) => string;

/** Translates a generic key, or the game's own wording of it when its UI module overrides the key. */
export const gameText =
  (t: ExtT, ui: Pick<GameUi, 'wording'>): GameText =>
  (key, values) =>
    ui.wording[key]?.(t, values) ?? t(key, values);
