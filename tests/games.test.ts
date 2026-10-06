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
