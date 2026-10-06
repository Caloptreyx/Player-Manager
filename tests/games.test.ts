import assert from 'node:assert/strict';
import { describe, test } from 'node:test';
import { gameText, gameUi } from '../frontend/src/games/index.ts';
import type { ExtT } from '../frontend/src/translations.ts';
import { javaGame, spec } from './fixtures.ts';

const NOTCH = '069a79f4-44e9-4726-a5be-fca90e38aaf5';

// echoes the key and its values, so the tests see which text a lookup resolved to
const t = ((key: string, values: Record<string, unknown>) =>
  Object.keys(values).length > 0 ? `${key} ${JSON.stringify(values)}` : key) as ExtT;

describe('gameUi', () => {
  test('an unknown game gets its id as name and the generic UI', () => {
    const ui = gameUi('terraria');
    assert.equal(ui.name, 'terraria');
    assert.equal(ui.avatarUrl({ name: 'Guide', id: '1' }), null);
    assert.equal(ui.levelLabel(t, 'admin', 'badge'), 'admin');
    assert.equal(ui.isOperatorLevel('admin'), true);
    assert.equal(gameText(t, ui)('lists.whitelist.title', {}), 'lists.whitelist.title');
  });

  test('ids that are object properties are not games', () => {
    assert.equal(gameUi('constructor').name, 'constructor');
    assert.equal(gameUi('toString').avatarUrl({ name: 'x', id: null }), null);
  });

  test('java avatars prefer the undashed uuid and fall back to the encoded name', () => {
    const { avatarUrl } = gameUi('minecraft_java');
    assert.equal(avatarUrl({ name: 'Notch', id: NOTCH }), `https://mc-heads.net/avatar/${NOTCH.replaceAll('-', '')}/32`);
    assert.equal(avatarUrl({ name: '.Floodgate', id: null }), 'https://mc-heads.net/avatar/.Floodgate/32');
    assert.equal(avatarUrl({ name: null, id: null }), null);
    assert.equal(avatarUrl({ name: 'Notch', id: null }, 96), 'https://mc-heads.net/avatar/Notch/96');
  });

  test('bedrock has no avatar service', () => {
    assert.equal(gameUi('minecraft_bedrock').avatarUrl({ name: 'Gamer', id: '123' }), null);
  });
});

describe('wording', () => {
  test('bedrock words the whitelist as allowlist and keeps the values', () => {
    const text = gameText(t, gameUi('minecraft_bedrock'));
    assert.equal(text('lists.whitelist.title', {}), 'games.minecraftBedrock.allowlist.title');
    assert.equal(
      text('lists.whitelist.added', { name: 'Gamer' }),
      'games.minecraftBedrock.allowlist.added {"name":"Gamer"}',
    );
    assert.equal(text('lists.operators.title', {}), 'lists.operators.title');
  });
});

describe('levels and notes', () => {
  test('only bedrock operators have operator rights', () => {
    const bedrock = gameUi('minecraft_bedrock');
    assert.equal(bedrock.isOperatorLevel('operator'), true);
    assert.equal(bedrock.isOperatorLevel('member'), false);
    assert.equal(bedrock.levelLabel(t, 'visitor', 'badge'), 'games.minecraftBedrock.levels.visitor');
  });

  test('java levels read as op levels on badges and describe themselves in the select', () => {
    const java = gameUi('minecraft_java');
    assert.equal(java.levelLabel(t, '3', 'badge'), 'games.minecraftJava.opLevel {"level":"3"}');
    assert.equal(java.levelLabel(t, '3', 'option'), 'games.minecraftJava.levels.3');
  });

  test('bedrock notes its missing ban list; other games have no notes', () => {
    assert.deepEqual(gameUi('minecraft_bedrock').notes(t), ['games.minecraftBedrock.noBans']);
    assert.deepEqual(gameUi('minecraft_java').notes(t), []);
    assert.deepEqual(gameUi('terraria').notes(t), []);
  });

  test('java explains why a running server offers no operator options', () => {
    const { addFormNote } = gameUi('minecraft_java');
    assert.equal(addFormNote(t, spec(javaGame('running'), 'operators')), 'games.minecraftJava.commandOptions');
    assert.equal(addFormNote(t, spec(javaGame('offline'), 'operators')), null);
    assert.equal(addFormNote(t, spec(javaGame('running'), 'bans')), null);
  });
});

describe('minecraft java profiles', () => {
  const { profile } = gameUi('minecraft_java');

  test('vanilla item icons come from the icon CDN by id; modded ids get none', () => {
    assert.equal(
      profile.itemIcon('minecraft:diamond_sword'),
      'https://mc.nerothe.com/img/1.26.2/minecraft_diamond_sword.png',
    );
    assert.equal(profile.itemIcon('oak_log'), 'https://mc.nerothe.com/img/1.26.2/minecraft_oak_log.png');
    assert.equal(profile.itemIcon('create:brass_ingot'), null);
    assert.equal(profile.itemIcon('minecraft:../secret'), null);
  });

  test('mobs show their spawn egg, general stats no icon', () => {
    assert.equal(profile.statIcon('minecraft:mined', 'minecraft:stone'), profile.itemIcon('minecraft:stone'));
    assert.equal(
      profile.statIcon('minecraft:killed', 'minecraft:zombie'),
      profile.itemIcon('minecraft:zombie_spawn_egg'),
    );
    assert.equal(
      profile.statIcon('minecraft:killed_by', 'minecraft:creeper'),
      profile.itemIcon('minecraft:creeper_spawn_egg'),
    );
    assert.equal(profile.statIcon('minecraft:custom', 'minecraft:jump'), null);
  });

  test('general stats are counted in ticks, centimeters and tenths of health; the rest are counts', () => {
    assert.equal(profile.statFormat('minecraft:custom', 'minecraft:play_time'), 'ticks');
    assert.equal(profile.statFormat('minecraft:custom', 'minecraft:time_since_death'), 'ticks');
    assert.equal(profile.statFormat('minecraft:custom', 'minecraft:walk_one_cm'), 'centimeters');
    assert.equal(profile.statFormat('minecraft:custom', 'minecraft:damage_dealt'), 'tenthHealth');
    assert.equal(profile.statFormat('minecraft:custom', 'minecraft:jump'), 'count');
    assert.equal(profile.statFormat('minecraft:mined', 'minecraft:walk_one_cm'), 'count');
    assert.equal(profile.statLabel('minecraft:walk_one_cm'), 'Walk');
    assert.equal(profile.statLabel('minecraft:play_one_minute'), 'Play Time');
  });

  test('highlights read play time from either key and add up flying', () => {
    const value = (stats: Record<string, Record<string, number>>, key: string) =>
      profile.statHighlights(t, stats).find((highlight) => highlight.key === key)?.value;
    assert.equal(value({ 'minecraft:custom': { 'minecraft:play_time': 72_000 } }, 'playTime'), 72_000);
    assert.equal(value({ 'minecraft:custom': { 'minecraft:play_one_minute': 1200 } }, 'playTime'), 1200);
    const custom = { 'minecraft:fly_one_cm': 500, 'minecraft:aviate_one_cm': 250, 'minecraft:deaths': 3 };
    assert.equal(value({ 'minecraft:custom': custom }, 'flown'), 750);
    assert.equal(value({ 'minecraft:custom': custom }, 'deaths'), 3);
    assert.equal(value({}, 'mobKills'), 0);
    assert.equal(profile.statHighlights(t, {})[0].label, 'games.minecraftJava.stats.playTime');
  });

  test('categories follow the statistics screen; unknown ones are appended by id', () => {
    const present = ['minecraft:custom', 'mymod:crafted_magic', 'minecraft:killed', 'minecraft:mined'];
    assert.deepEqual(profile.statCategories(t, present), [
      { id: 'minecraft:mined', label: 'games.minecraftJava.statCategories.mined' },
      { id: 'minecraft:killed', label: 'games.minecraftJava.statCategories.killed' },
      { id: 'minecraft:custom', label: 'games.minecraftJava.statCategories.custom' },
      { id: 'mymod:crafted_magic', label: 'Crafted Magic' },
    ]);
  });

  test('vanilla dimensions are named, custom worlds prettified', () => {
    assert.equal(profile.dimensionLabel(t, 'minecraft:the_nether'), 'games.minecraftJava.dimensions.the_nether');
    assert.equal(profile.dimensionLabel(t, 'mymod:sky_islands'), 'Sky Islands');
  });

  test('the body render prefers the undashed uuid', () => {
    assert.equal(
      profile.bodyUrl({ name: 'Notch', id: NOTCH }, 320),
      `https://mc-heads.net/body/${NOTCH.replaceAll('-', '')}/320/right`,
    );
    assert.equal(gameUi('minecraft_bedrock').profile.bodyUrl({ name: 'Gamer', id: '1' }, 320), null);
  });
});
