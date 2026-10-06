import {
  type DefinedTranslations,
  defineEnglishItem,
  defineTranslations,
  type GetPlaceholders,
  type PathValue,
} from 'shared';

const translations = defineTranslations({
  items: {
    player: defineEnglishItem('player', 'players'),
  },
  translations: {
    // generic wording; a game module may replace any of it with a key of its own under `games`
    common: {
      players: 'Players',
      playerManager: 'Player Manager',
      subtitle: 'See who is online and manage the whitelist, operators and bans of your game server.',
      refresh: 'Refresh',
      retry: 'Retry',
      moreActions: 'More actions',
      unknownPlayer: 'Unknown player',
      copyId: 'Click to copy',
    },
    states: {
      offline: 'Offline',
      starting: 'Starting',
      stopping: 'Stopping',
      running: 'Running',
    },
    header: {
      onlineMode: 'Online mode',
      offlineMode: 'Offline mode',
      onlineModeHint: 'Players sign in with a verified account.',
      offlineModeHint: 'Accounts are not verified; anyone can join with any name.',
      on: 'On',
      off: 'Off',
      restartRequired: 'Restart the server to apply the {list} change; the server only reads this setting on start.',
    },
    // how list changes are carried out: a short chip label and its explanation
    method: {
      command: 'Console commands',
      commandHint: 'The server is running, so changes are sent as console commands and the server updates its files.',
      file: 'File edits',
      fileHint: 'Changes are written straight to the server files.',
      fileReload: 'File edits + reload',
      fileReloadHint: 'Changes are written to the files and reloaded with a console command.',
      transition: 'Changes are paused while the server is {state}.',
      notRunning: 'Start the server to change the lists.',
    },
    blockers: {
      transition: 'Wait until the server has finished starting or stopping.',
      notRunning: 'Only available while the server is running.',
      noConsole: 'You need permission to send console commands to change this while the server is running.',
      noReadConsole: 'You need permission to read the console for this.',
      noFiles: 'You need permission to edit files to change this.',
      noPermission: 'You do not have permission to change this.',
    },
    badges: {
      bypassesLimit: 'Bypasses player limit',
    },
    tiles: {
      banned: 'Banned',
      ips: '+{count} IP',
    },
    online: {
      tab: 'Online',
      count: '{count} of {max} players online',
      checked: 'checked {time}',
      hint: 'Every refresh runs the "list" command in the server console.',
      emptyTitle: 'Nobody is online',
      empty: 'Players show up here as soon as they join.',
      unavailableTitle: 'Player list unavailable',
      noPermission: 'You need permission to send and read console commands to see who is online.',
      offlineTitle: 'Server not running',
      offline: 'Start the server to see who is online.',
      kick: 'Kick',
      kickTitle: 'Kick {name}',
      kickContent: '{name} is disconnected from the server and can join again right away.',
      kicked: '{name} kicked.',
      banTitle: 'Ban {name}',
      banContent: '{name} is disconnected and cannot join again until unbanned.',
    },
    lists: {
      search: 'Search by name, ID or IP',
      noMatches: 'Nothing matches "{query}".',
      clearSearch: 'Clear search',
      player: 'Player',
      address: 'IP address',
      status: 'Status',
      details: 'Details',
      actions: 'Actions',
      removeTitle: 'Remove {name}',
      source: 'by {source}',
      expires: 'expires {expires}',
      whitelist: {
        title: 'Whitelist',
        empty: 'The whitelist is empty',
        emptyDescription: 'While the whitelist is on, only the players on it can join.',
        add: 'Add player',
        addTitle: 'Add to whitelist',
        addOnline: 'Add to whitelist',
        submit: 'Add',
        remove: 'Remove from whitelist',
        removeContent: 'Remove {name} from the whitelist?',
        removeConfirm: 'Remove',
        added: '{name} added to the whitelist.',
        removed: '{name} removed from the whitelist.',
        badge: 'Whitelisted',
        on: 'Whitelist turned on.',
        off: 'Whitelist turned off.',
      },
      operators: {
        title: 'Operators',
        empty: 'No operators yet',
        emptyDescription: 'Operators can run admin commands on the server.',
        add: 'Add operator',
        addTitle: 'Add operator',
        addOnline: 'Make operator',
        submit: 'Add',
        remove: 'Remove operator',
        removeContent: 'Take the operator permissions away from {name}?',
        removeConfirm: 'Remove',
        added: '{name} is now an operator.',
        removed: '{name} is no longer an operator.',
        badge: 'Operator',
      },
      bans: {
        title: 'Bans',
        empty: 'Nobody is banned',
        emptyDescription: 'Banned players cannot join the server until they are unbanned.',
        add: 'Ban player',
        addTitle: 'Ban player',
        addOnline: 'Ban',
        submit: 'Ban',
        remove: 'Unban',
        removeContent: 'Unban {name}? They can join the server again.',
        removeConfirm: 'Unban',
        added: '{name} banned.',
        removed: '{name} unbanned.',
        badge: 'Banned',
      },
      ip_bans: {
        title: 'IP bans',
        empty: 'No banned addresses',
        emptyDescription: 'Players connecting from a banned IP address are turned away.',
        add: 'Ban IP',
        addTitle: 'Ban IP',
        submit: 'Ban',
        remove: 'Unban',
        removeContent: 'Unban {name}? Players from this address can join again.',
        removeConfirm: 'Unban',
        added: '{name} banned.',
        removed: '{name} unbanned.',
      },
    },
    form: {
      name: 'Name',
      id: 'Player ID',
      advanced: 'Advanced',
      idOptional: 'Optional, looked up from the name when left empty.',
      knownId: 'Known: {id}',
      level: 'Permission level',
      bypassesLimit: 'Can join when the server is full.',
      reason: 'Reason',
      reasonPlaceholder: 'Optional',
      reasonCount: '{count} / {max}',
      ip: 'IP address',
    },
    errors: {
      required: 'Required.',
      name: 'This is not a valid player name.',
      id: 'This is not a valid player id.',
      reasonLength: 'Keep the reason under 256 characters.',
      reasonControl: 'The reason cannot contain line breaks or control characters.',
      ip: 'Enter a valid IPv4 or IPv6 address.',
      duplicate: 'Already on this list.',
      filesTitle: 'Some player files could not be read',
      filesContent: 'Changes to these lists may fail until the files are fixed.',
      loadTitle: 'Could not load the players',
    },
    notDetected: {
      title: 'No supported game detected',
      description: 'This server does not look like any supported game ({games}), so there are no players to manage.',
    },

    // per-game overrides and texts, used by the modules in `games/`
    games: {
      minecraftJava: {
        uuid: 'UUID',
        nameError: 'Use 1-16 letters, digits or underscores.',
        uuidError: 'Enter a UUID, with or without dashes.',
        opLevel: 'Op {level}',
        levels: {
          '1': '1: bypass spawn protection',
          '2': '2: cheat commands, command blocks',
          '3': '3: multiplayer management (kick, ban)',
          '4': '4: all commands, incl. stop',
        },
        commandOptions: 'The level and player limit options can only be set while the server is offline.',
      },
      minecraftBedrock: {
        gamertag: 'Gamertag',
        xuid: 'XUID',
        gamertagError: 'Use 1-32 letters, digits or spaces, not starting or ending with a space.',
        xuidError: 'Enter the XUID: 1-20 digits.',
        ignoresLimit: 'Ignores player limit',
        allowlist: {
          title: 'Allowlist',
          empty: 'The allowlist is empty',
          emptyDescription: 'While the allowlist is on, only the players on it can join.',
          addTitle: 'Add to allowlist',
          remove: 'Remove from allowlist',
          removeContent: 'Remove {name} from the allowlist?',
          added: '{name} added to the allowlist.',
          removed: '{name} removed from the allowlist.',
          badge: 'Allowlisted',
          on: 'Allowlist turned on.',
          off: 'Allowlist turned off.',
        },
        levels: {
          operator: 'Operator',
          member: 'Member',
          visitor: 'Visitor',
        },
        noBans:
          'Bedrock Dedicated Server has no ban list. Turn on the allowlist to keep players out, or kick them while they are online.',
      },
    },
  },
});

export const useExtTranslations = translations.useTranslations.bind(translations);
export const getExtTranslations = translations.getTranslations.bind(translations);

type Messages = (typeof translations)['obj'];
/** Every translation key of the extension. */
export type ExtKey = typeof translations extends DefinedTranslations<infer _I, infer _O, infer P> ? P : never;
/** The placeholder values a key needs. */
export type ExtValues<K extends ExtKey> = Record<
  GetPlaceholders<PathValue<Messages, K> & string>[number],
  string | number
>;
/** `t` of `useExtTranslations`, for code outside components (the game modules). */
export type ExtT = <K extends ExtKey>(key: K, values: ExtValues<K>) => string;
/** Keys a game module may override: everything outside its own `games` texts. */
export type GenericKey = Exclude<ExtKey, `games.${string}`>;
/** Replacements of generic texts by a game, rendered with the values of the generic key. */
export type Wording = { [K in GenericKey]?: (t: ExtT, values: ExtValues<K>) => string };

export default translations;
