import assert from 'node:assert/strict';
import { describe, test } from 'node:test';
import type { OnlinePlayers } from '../frontend/src/lib/model.ts';
import {
  knownId,
  matchesFilter,
  ONLINE_REFRESH_MS,
  onlineRefreshInterval,
  playerStatus,
  refetchDelay,
  sortBy,
  suggestions,
  withoutPlayer,
} from '../frontend/src/lib/players.ts';
import { entry } from './fixtures.ts';

const NOTCH = '069a79f4-44e9-4726-a5be-fca90e38aaf5';

describe('playerStatus', () => {
  const lists = {
    whitelist: [entry({ name: 'Notch', id: NOTCH })],
    operators: [entry({ id: '2535416409688276', level: 'operator' })],
    bans: [entry({ name: 'Griefer' })],
    ip_bans: [entry({ ip: '10.0.0.1' })],
  };

  test('matches ids regardless of dashes and case', () => {
    const status = playerStatus(lists, { name: 'RenamedNotch', id: NOTCH.replaceAll('-', '').toUpperCase() });
    assert.equal(status.whitelist?.name, 'Notch');
  });

  test('two different ids do not match even with the same name', () => {
    assert.equal(playerStatus(lists, { name: 'Notch', id: '00000000-0000-0000-0000-000000000000' }).whitelist, undefined);
  });

  test('falls back to case-insensitive names when an id is missing', () => {
    assert.ok(playerStatus(lists, { name: 'notch', id: null }).whitelist);
    assert.ok(playerStatus(lists, { name: 'GRIEFER', id: '1' }).bans);
  });

  test('operators without a name match by id only', () => {
    assert.equal(playerStatus(lists, { name: 'Gamer', id: '2535416409688276' }).operators?.level, 'operator');
    assert.equal(playerStatus(lists, { name: 'Gamer', id: null }).operators, undefined);
  });

  test('lists the game does not have are absent', () => {
    assert.deepEqual(playerStatus({ whitelist: [] }, { name: 'Notch', id: null }), {});
  });
});

describe('sortBy', () => {
  test('sorts case-insensitively and numerically, unnamed entries last by id', () => {
    const sorted = sortBy(
      [
        entry({ name: 'bob' }),
        entry({ id: '20' }),
        entry({ name: 'Alice' }),
        entry({ id: '3' }),
        entry({ name: 'player10' }),
        entry({ name: 'player9' }),
      ],
      (row) => row.name,
      (row) => row.id,
    );
    assert.deepEqual(
      sorted.map((row) => row.name ?? row.id),
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

const onlineList = (fields: Partial<OnlinePlayers>): OnlinePlayers => ({
  count: 0,
  max: 20,
  players: [],
  source: 'query',
  complete: true,
  ...fields,
});

describe('withoutPlayer', () => {
  test('removes the kicked player and lowers the count', () => {
    const online = onlineList({
      count: 2,
      players: [
        { name: 'Notch', id: null },
        { name: 'jeb_', id: null },
      ],
    });
    assert.deepEqual(withoutPlayer(online, 'notch'), { ...online, count: 1, players: [{ name: 'jeb_', id: null }] });
  });

  test('an unknown name leaves the list unchanged', () => {
    const online = onlineList({ count: 1, players: [{ name: 'Notch', id: null }] });
    assert.deepEqual(withoutPlayer(online, 'Herobrine'), online);
  });
});

describe('onlineRefreshInterval', () => {
  test('answers that did not touch the console refresh by themselves', () => {
    for (const source of ['query', 'rcon', 'ping'] as const) {
      assert.equal(onlineRefreshInterval(onlineList({ source }), false), ONLINE_REFRESH_MS);
    }
    assert.equal(onlineRefreshInterval(onlineList({ source: 'ping', complete: false }), false), ONLINE_REFRESH_MS);
  });

  test('console answers, failed fetches and missing answers do not', () => {
    assert.equal(onlineRefreshInterval(onlineList({ source: 'console' }), false), false);
    assert.equal(onlineRefreshInterval(onlineList({ source: 'query' }), true), false);
    assert.equal(onlineRefreshInterval(undefined, false), false);
  });
});

describe('refetchDelay', () => {
  test('waits for the server to write its files after a console command only', () => {
    assert.ok(refetchDelay('command') > 0);
    assert.equal(refetchDelay('file'), 0);
  });
});
