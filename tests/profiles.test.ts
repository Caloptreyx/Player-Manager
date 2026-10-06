import assert from 'node:assert/strict';
import { describe, test } from 'node:test';
import type { Access } from '../frontend/src/lib/access.ts';
import type { Item, OnlinePlayers } from '../frontend/src/lib/model.ts';
import {
  ARMOR_SLOTS,
  ENDER_CHEST_SLOTS,
  formatCentimeters,
  formatStat,
  formatTicksClock,
  formatTicksDuration,
  HOTBAR_SLOTS,
  iconFills,
  idInitials,
  MAIN_SLOTS,
  parseStoredDate,
  prettifyId,
  prettySnbt,
  profileActionAccess,
  profileMode,
  romanNumeral,
  seenItemIds,
  slotItems,
} from '../frontend/src/lib/profiles.ts';

const NOTCH = '069a79f4-44e9-4726-a5be-fca90e38aaf5';

const item = (slot: string, id = 'minecraft:stone'): Item => ({
  slot,
  id,
  count: 1,
  name: null,
  enchantments: [],
  damage: null,
  snbt: `{id:"${id}"}`,
});

describe('slot layout', () => {
  test('the inventory grids hold the command slot names of the contract', () => {
    assert.deepEqual(ARMOR_SLOTS, ['armor.head', 'armor.chest', 'armor.legs', 'armor.feet']);
    assert.equal(MAIN_SLOTS.length, 27);
    assert.equal(MAIN_SLOTS[0], 'inventory.0');
    assert.equal(MAIN_SLOTS[26], 'inventory.26');
    assert.deepEqual(
      HOTBAR_SLOTS,
      Array.from({ length: 9 }, (_, index) => `hotbar.${index}`),
    );
    assert.equal(ENDER_CHEST_SLOTS[26], 'enderchest.26');
  });

  test('items go to their slot; unknown and doubled slots are kept apart instead of dropped', () => {
    const sword = item('hotbar.0', 'minecraft:diamond_sword');
    const helmet = item('armor.head', 'minecraft:iron_helmet');
    const shield = item('weapon.offhand', 'minecraft:shield');
    const modded = item('curios.ring', 'mymod:ring');
    const doubled = item('hotbar.0', 'minecraft:dirt');
    const { bySlot, unplaced } = slotItems([sword, helmet, shield, modded, doubled], 'inventory');
    assert.equal(bySlot.get('hotbar.0'), sword);
    assert.equal(bySlot.get('armor.head'), helmet);
    assert.equal(bySlot.get('weapon.offhand'), shield);
    assert.deepEqual(unplaced, [modded, doubled]);
  });

  test('an ender chest has no place for inventory slots', () => {
    const chest = item('enderchest.3');
    const stray = item('hotbar.1');
    const { bySlot, unplaced } = slotItems([chest, stray], 'ender_chest');
    assert.equal(bySlot.get('enderchest.3'), chest);
    assert.deepEqual(unplaced, [stray]);
  });

  test('the give suggestions are the distinct ids of both containers', () => {
    const profile = {
      inventory: [item('hotbar.0', 'minecraft:torch'), item('hotbar.1', 'minecraft:apple')],
      ender_chest: [item('enderchest.0', 'minecraft:torch')],
    };
    assert.deepEqual(seenItemIds(profile), ['minecraft:apple', 'minecraft:torch']);
  });
});

describe('romanNumeral', () => {
  test('levels read like the game shows them', () => {
    assert.deepEqual([1, 2, 3, 4, 5, 9, 10, 14, 40, 90, 255].map(romanNumeral), [
      'I',
      'II',
      'III',
      'IV',
      'V',
      'IX',
      'X',
      'XIV',
      'XL',
      'XC',
      'CCLV',
    ]);
    assert.equal(romanNumeral(1994), 'MCMXCIV');
  });

  test('levels without a numeral stay digits', () => {
    assert.equal(romanNumeral(0), '0');
    assert.equal(romanNumeral(-3), '-3');
    assert.equal(romanNumeral(4000), '4000');
    assert.equal(romanNumeral(2.5), '2.5');
  });
});

describe('durations', () => {
  test('effects count down as a clock, infinite ones have none', () => {
    assert.equal(formatTicksClock(19), '0:00');
    assert.equal(formatTicksClock(1200), '1:00');
    assert.equal(formatTicksClock(20 * 185), '3:05');
    assert.equal(formatTicksClock(20 * 3725), '1:02:05');
    assert.equal(formatTicksClock(-1), null);
  });

  test('play time shows its two largest units', () => {
    assert.equal(formatTicksDuration(20 * (3 * 86_400 + 4 * 3600 + 59 * 60), 'en'), '3d 4h');
    assert.equal(formatTicksDuration(20 * (12 * 60 + 5), 'en'), '12m 5s');
    assert.equal(formatTicksDuration(20 * 3600, 'en'), '1h');
    // the second unit is the one after the largest, even when it is zero
    assert.equal(formatTicksDuration(20 * (86_400 + 30), 'en'), '1d');
    assert.equal(formatTicksDuration(0, 'en'), '0s');
  });
});

describe('stat values', () => {
  test('distances in centimeters read in m below a kilometer, in km above', () => {
    assert.equal(formatCentimeters(45_678, 'en'), '457 m');
    assert.equal(formatCentimeters(99_940, 'en'), '999 m');
    assert.equal(formatCentimeters(99_960, 'en'), '1 km');
    assert.equal(formatCentimeters(123_456, 'en'), '1.2 km');
  });

  test('each format converts its unit', () => {
    assert.equal(formatStat('count', 1_234_567, 'en'), '1,234,567');
    assert.equal(formatStat('ticks', 20 * 90, 'en'), '1m 30s');
    assert.equal(formatStat('centimeters', 250, 'en'), '3 m');
    // tenths of a health point; a heart is two points
    assert.equal(formatStat('tenthHealth', 1255, 'en'), '62.8 ❤');
    assert.equal(formatStat('tenthHealth', 20, 'en'), '1 ❤');
  });
});

describe('ids', () => {
  test('ids read as words of their last path segment', () => {
    assert.equal(prettifyId('minecraft:diamond_sword'), 'Diamond Sword');
    assert.equal(prettifyId('minecraft:story/mine_stone'), 'Mine Stone');
    assert.equal(prettifyId('create:brass_ingot'), 'Brass Ingot');
    assert.equal(prettifyId('stone'), 'Stone');
    assert.equal(prettifyId('minecraft:'), 'minecraft:');
  });

  test('the fallback tile shows the initials of the words, or the first two letters', () => {
    assert.equal(idInitials('mymod:copper_wire'), 'CW');
    assert.equal(idInitials('mymod:gear'), 'GE');
    assert.equal(idInitials('mymod:tools/big.hammer'), 'BH');
    assert.equal(idInitials('mymod:'), '?');
  });

  test('advancement timestamps parse in the stored format and ISO', () => {
    assert.equal(parseStoredDate('2024-05-01 12:34:56 +0200')?.toISOString(), '2024-05-01T10:34:56.000Z');
    assert.equal(parseStoredDate('2024-05-01 12:34:56 -0130')?.toISOString(), '2024-05-01T14:04:56.000Z');
    assert.equal(parseStoredDate('2024-05-01T12:00:00Z')?.toISOString(), '2024-05-01T12:00:00.000Z');
    assert.equal(parseStoredDate('yesterday'), null);
  });
});

describe('iconFills', () => {
  test('hearts fill in halves and round up like the HUD', () => {
    assert.deepEqual(iconFills(20, 20), Array(10).fill(1));
    assert.deepEqual(iconFills(7, 20), [1, 1, 1, 0.5, 0, 0, 0, 0, 0, 0]);
    assert.deepEqual(iconFills(6.2, 20), [1, 1, 1, 0.5, 0, 0, 0, 0, 0, 0]);
    assert.deepEqual(iconFills(0, 20), Array(10).fill(0));
  });

  test('extra max health adds icons; values stay within the bar', () => {
    assert.equal(iconFills(30, 30).length, 15);
    assert.deepEqual(iconFills(25, 6), [1, 1, 1]);
    assert.deepEqual(iconFills(-4, 4), [0, 0]);
  });
});

describe('prettySnbt', () => {
  test('one entry per line, short flat compounds inline, quoted strings untouched', () => {
    const snbt =
      '{count:1,id:"minecraft:diamond_sword",components:{"minecraft:enchantments":{levels:{"minecraft:sharpness":5}},' +
      `"minecraft:custom_name":'{"text":"Blade, of {doom}"}'}}`;
    assert.equal(
      prettySnbt(snbt),
      [
        '{',
        '  count: 1,',
        '  id: "minecraft:diamond_sword",',
        '  components: {',
        '    "minecraft:enchantments": {',
        '      levels: {"minecraft:sharpness": 5}',
        '    },',
        `    "minecraft:custom_name": '{"text":"Blade, of {doom}"}'`,
        '  }',
        '}',
      ].join('\n'),
    );
  });

  test('typed arrays and empty lists stay on one line; escaped quotes do not end a string', () => {
    assert.equal(
      prettySnbt('{Data:[I;1,2,3],Lore:[],Name:"say \\"hi\\", {x}"}'),
      ['{', '  Data: [I;1, 2, 3],', '  Lore: [],', '  Name: "say \\"hi\\", {x}"', '}'].join('\n'),
    );
  });
});

describe('profile actions', () => {
  const online = (fields: Partial<OnlinePlayers>): OnlinePlayers => ({
    count: 1,
    max: 20,
    players: [{ name: 'Notch', id: null }],
    source: 'query',
    complete: true,
    ...fields,
  });
  const notch = { name: 'Notch', id: NOTCH };

  test('the mode follows the server state and the online list like the backend', () => {
    assert.equal(profileMode('offline', undefined, notch), 'file');
    assert.equal(profileMode('starting', online({}), notch), 'transition');
    assert.equal(profileMode('stopping', online({}), notch), 'transition');
    assert.equal(profileMode('running', undefined, notch), 'unknown');
    assert.equal(profileMode('running', online({}), { name: 'notch', id: null }), 'live');
    // a dashless id matches; names may differ (renamed player)
    const byId = online({ players: [{ name: 'OldName', id: NOTCH.replaceAll('-', '').toUpperCase() }] });
    assert.equal(profileMode('running', byId, notch), 'live');
    assert.equal(profileMode('running', online({ players: [] }), notch), 'file');
    assert.equal(profileMode('running', online({ players: [], complete: false, count: 3 }), notch), 'unknown');
  });

  const usable: Access = { visible: true, blocker: null };
  const noConsole: Access = { visible: false, blocker: 'noConsole' };
  const noFiles: Access = { visible: false, blocker: 'noFiles' };
  const notRunning: Access = { visible: true, blocker: 'notRunning' };

  test('edits use the capability of the mode', () => {
    assert.deepEqual(profileActionAccess('live', usable, noFiles, false), usable);
    assert.deepEqual(profileActionAccess('live', noConsole, usable, false), { visible: true, blocker: 'noConsole' });
    assert.deepEqual(profileActionAccess('file', notRunning, usable, false), usable);
    assert.deepEqual(profileActionAccess('file', usable, noFiles, false), { visible: true, blocker: 'noFiles' });
    assert.deepEqual(profileActionAccess('transition', usable, usable, false), {
      visible: true,
      blocker: 'transition',
    });
  });

  test('while the mode is unknown either capability will do', () => {
    assert.deepEqual(profileActionAccess('unknown', noConsole, usable, false), usable);
    assert.deepEqual(profileActionAccess('unknown', usable, noFiles, false), usable);
    assert.deepEqual(profileActionAccess('unknown', noConsole, noFiles, false), {
      visible: false,
      blocker: 'noConsole',
    });
  });

  test('giving needs the player online and the console', () => {
    assert.deepEqual(profileActionAccess('live', usable, usable, true), usable);
    assert.deepEqual(profileActionAccess('unknown', usable, usable, true), usable);
    assert.deepEqual(profileActionAccess('file', usable, usable, true), { visible: true, blocker: 'playerOffline' });
    assert.equal(profileActionAccess('live', noConsole, usable, true).visible, false);
  });
});
