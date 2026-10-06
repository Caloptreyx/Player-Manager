import type { GameModule } from './index.ts';
import minecraftJavaProfile from './minecraftJavaProfile.ts';

const LEVELS = ['1', '2', '3', '4'] as const;
const isLevel = (level: string): level is (typeof LEVELS)[number] => (LEVELS as readonly string[]).includes(level);

const minecraftJava: GameModule = {
  name: 'Minecraft Java',
  color: 'orange',
  profile: minecraftJavaProfile,
  // mc-heads renders the skin of a uuid (preferred, undashed) or a name
  avatarUrl: (player, size = 32) => {
    const key = player.id ? player.id.replaceAll('-', '').toLowerCase() : player.name;
    return key ? `https://mc-heads.net/avatar/${encodeURIComponent(key)}/${size}` : null;
  },
  wording: {
    'form.id': (t) => t('games.minecraftJava.uuid', {}),
    'errors.name': (t) => t('games.minecraftJava.nameError', {}),
    'errors.id': (t) => t('games.minecraftJava.uuidError', {}),
  },
  levelLabel: (t, level, place) => {
    if (place === 'badge') return t('games.minecraftJava.opLevel', { level });
    return isLevel(level) ? t(`games.minecraftJava.levels.${level}`, {}) : level;
  },
  // console commands (`op <name>`) take no level or player limit flag, so the list spec offers none
  addFormNote: (t, spec) =>
    spec.kind === 'operators' && spec.levels === null ? t('games.minecraftJava.commandOptions', {}) : null,
};

export default minecraftJava;
