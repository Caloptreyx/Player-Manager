import assert from 'node:assert/strict';
import { describe, test } from 'node:test';
import {
  avatarUrl,
  knownId,
  matchesFilter,
  type Operator,
  playerStatus,
  refetchDelay,
  sortBy,
  suggestions,
  withoutPlayer,
} from '../frontend/src/lib/players.ts';

const NOTCH = '069a79f4-44e9-4726-a5be-fca90e38aaf5';

const op = (name: string | null, id: string | null, level: Operator['level']): Operator => ({
  name,
  id,
  level,
  bypasses_player_limit: null,
});

describe('playerStatus', () => {
  const lists = {
    whitelist: [{ name: 'Notch', id: NOTCH, ignores_player_limit: null }],
    operators: [op(null, '2535416409688276', 'operator')],
    bans: [{ name: 'Griefer', id: null, reason: null, source: null, created: null, expires: null }],
  };

  test('matches ids regardless of dashes and case', () => {
    const status = playerStatus(lists, { name: 'RenamedNotch', id: NOTCH.replaceAll('-', '').toUpperCase() });
    assert.equal(status.whitelisted, true);
  });

  test('two different ids do not match even with the same name', () => {
    assert.equal(playerStatus(lists, { name: 'Notch', id: '00000000-0000-0000-0000-000000000000' }).whitelisted, false);
  });

  test('falls back to case-insensitive names when an id is missing', () => {
    assert.equal(playerStatus(lists, { name: 'notch', id: null }).whitelisted, true);
    assert.equal(playerStatus(lists, { name: 'GRIEFER', id: '1' }).banned, true);
  });

  test('bedrock operators without a name match by xuid only', () => {
    assert.equal(playerStatus(lists, { name: 'Gamer', id: '2535416409688276' }).operator?.level, 'operator');
    assert.equal(playerStatus(lists, { name: 'Gamer', id: null }).operator, null);
  });
});

describe('avatarUrl', () => {
  test('java prefers the undashed uuid and falls back to the encoded name', () => {
    assert.equal(avatarUrl('java', { name: 'Notch', id: NOTCH }), `https://mc-heads.net/avatar/${NOTCH.replaceAll('-', '')}/32`);
    assert.equal(avatarUrl('java', { name: '.Floodgate', id: null }), 'https://mc-heads.net/avatar/.Floodgate/32');
    assert.equal(avatarUrl('java', { name: null, id: null }), null);
  });

  test('bedrock has no avatar service', () => {
    assert.equal(avatarUrl('bedrock', { name: 'Gamer', id: '123' }), null);
  });
});

describe('sortBy', () => {
  test('sorts case-insensitively and numerically, unnamed entries last by id', () => {
    const sorted = sortBy(
      [op('bob', null, 4), op(null, '20', 'member'), op('Alice', null, 4), op(null, '3', 'member'), op('player10', null, 1), op('player9', null, 1)],
      (entry) => entry.name,
      (entry) => entry.id,
    );
    assert.deepEqual(
      sorted.map((entry) => entry.name ?? entry.id),
      ['Alice', 'bob', 'player9', 'player10', '3', '20'],
    );
  });
});

describe('suggestions and knownId', () => {
  const known = [
    { name: 'Notch', id: NOTCH },
    { name: 'jeb_', id: null },
    { name: 'JEB_', id: null },
    { name: 'Dinnerbone', id: '61699b2e-d327-4a01-9f1e-0ea8c3f06bc6' },
  ];

  test('drops players already on the list and case-duplicates', () => {
    assert.deepEqual(suggestions(known, [{ name: 'notch', id: null }]), ['Dinnerbone', 'jeb_']);
  });

  test('knownId finds the id case-insensitively and skips players without one', () => {
    assert.equal(knownId(known, 'NOTCH'), NOTCH);
    assert.equal(knownId(known, 'jeb_'), null);
  });
});

describe('matchesFilter', () => {
  test('substring match on any value, ignoring case and surrounding spaces', () => {
    assert.equal(matchesFilter(['Notch', null], '  NOT '), true);
    assert.equal(matchesFilter(['Notch', NOTCH], '44e9'), true);
    assert.equal(matchesFilter(['Notch', null], 'jeb'), false);
    assert.equal(matchesFilter([null], ''), true);
  });
});

describe('withoutPlayer', () => {
  test('removes the kicked player and lowers the count', () => {
    const online = { count: 2, max: 20, players: [{ name: 'Notch', id: null }, { name: 'jeb_', id: null }] };
    assert.deepEqual(withoutPlayer(online, 'notch'), { count: 1, max: 20, players: [{ name: 'jeb_', id: null }] });
  });

  test('an unknown name leaves the list unchanged', () => {
    const online = { count: 1, max: 20, players: [{ name: 'Notch', id: null }] };
    assert.deepEqual(withoutPlayer(online, 'Herobrine'), online);
  });
});

describe('refetchDelay', () => {
  test('waits for the server to write its files after a console command only', () => {
    assert.ok(refetchDelay('command') > 0);
    assert.equal(refetchDelay('file'), 0);
  });
});
