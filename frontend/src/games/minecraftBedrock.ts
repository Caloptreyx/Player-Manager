import type { GameModule } from './index.ts';

const LEVELS = ['operator', 'member', 'visitor'] as const;
const isLevel = (level: string): level is (typeof LEVELS)[number] => (LEVELS as readonly string[]).includes(level);

// Bedrock has no public skin service, so avatars fall back to initials
const minecraftBedrock: GameModule = {
  name: 'Minecraft Bedrock',
  color: 'teal',
  wording: {
    'form.name': (t) => t('games.minecraftBedrock.gamertag', {}),
    'form.id': (t) => t('games.minecraftBedrock.xuid', {}),
    'errors.name': (t) => t('games.minecraftBedrock.gamertagError', {}),
    'errors.id': (t) => t('games.minecraftBedrock.xuidError', {}),
    'badges.bypassesLimit': (t) => t('games.minecraftBedrock.ignoresLimit', {}),
    'lists.whitelist.title': (t) => t('games.minecraftBedrock.allowlist.title', {}),
    'lists.whitelist.empty': (t) => t('games.minecraftBedrock.allowlist.empty', {}),
    'lists.whitelist.addTitle': (t) => t('games.minecraftBedrock.allowlist.addTitle', {}),
    'lists.whitelist.addOnline': (t) => t('games.minecraftBedrock.allowlist.addTitle', {}),
    'lists.whitelist.remove': (t) => t('games.minecraftBedrock.allowlist.remove', {}),
    'lists.whitelist.removeContent': (t, values) => t('games.minecraftBedrock.allowlist.removeContent', values),
    'lists.whitelist.added': (t, values) => t('games.minecraftBedrock.allowlist.added', values),
    'lists.whitelist.removed': (t, values) => t('games.minecraftBedrock.allowlist.removed', values),
    'lists.whitelist.badge': (t) => t('games.minecraftBedrock.allowlist.badge', {}),
    'lists.whitelist.on': (t) => t('games.minecraftBedrock.allowlist.on', {}),
    'lists.whitelist.off': (t) => t('games.minecraftBedrock.allowlist.off', {}),
  },
  levelLabel: (t, level) => (isLevel(level) ? t(`games.minecraftBedrock.levels.${level}`, {}) : level),
  // members and visitors live in permissions.json too, but only operators have operator rights
  isOperatorLevel: (level) => level === 'operator',
  missingLists: {
    bans: {
      title: (t) => t('games.minecraftBedrock.noBansTitle', {}),
      description: (t) => t('games.minecraftBedrock.noBansDescription', {}),
    },
  },
};

export default minecraftBedrock;
