import assert from 'node:assert/strict';
import { describe, test } from 'node:test';
import { addBody, banDate, isDuplicate, removeBody } from '../frontend/src/lib/lists.ts';
import { bedrockGame, entry, javaGame, spec } from './fixtures.ts';

const NOTCH = '069a79f4-44e9-4726-a5be-fca90e38aaf5';
const everything = { name: 'Notch', id: NOTCH, ip: '10.0.0.1', level: '2', bypasses_player_limit: true, reason: 'x' };

describe('addBody', () => {
  test('console commands take the name only', () => {
    assert.deepEqual(JSON.parse(JSON.stringify(addBody(spec(javaGame('running'), 'operators'), everything))), {
      name: 'Notch',
    });
  });

  test('file edits take the id, level and player limit flag the spec offers', () => {
    assert.deepEqual(JSON.parse(JSON.stringify(addBody(spec(javaGame('offline'), 'operators'), everything))), {
      name: 'Notch',
      id: NOTCH,
      level: '2',
      bypasses_player_limit: true,
    });
  });

  test('a level left unchosen is the spec default', () => {
    assert.equal(addBody(spec(bedrockGame('running'), 'operators'), { name: 'Gamer' }).level, 'operator');
  });

  test('ip lists send the ip, and blank ids and reasons are left out', () => {
    const body = addBody(spec(javaGame('running'), 'ip_bans'), { ...everything, reason: '' });
    assert.deepEqual(JSON.parse(JSON.stringify(body)), { ip: '10.0.0.1' });
    assert.equal(addBody(spec(bedrockGame('offline'), 'whitelist'), { name: 'Gamer', id: '' }).id, undefined);
  });
});

describe('removeBody', () => {
  test('players by name and id, ip bans by ip', () => {
    const operators = spec(bedrockGame('running'), 'operators');
    assert.deepEqual(removeBody(operators, entry({ id: '2535416409688276', level: 'operator' })), {
      name: undefined,
      id: '2535416409688276',
    });
    assert.deepEqual(removeBody(spec(javaGame('running'), 'ip_bans'), entry({ ip: '10.0.0.1' })), { ip: '10.0.0.1' });
  });
});

describe('isDuplicate', () => {
  const entries = [entry({ name: 'Notch', id: NOTCH, level: '4' })];

  test('a player already on a list without levels is a duplicate', () => {
    assert.equal(isDuplicate(spec(javaGame('running'), 'operators'), entries, { name: 'notch', id: null }), true);
    assert.equal(isDuplicate(spec(javaGame('running'), 'operators'), entries, { name: 'jeb_', id: null }), false);
  });

  test('re-adding where a level can be chosen updates the level instead', () => {
    assert.equal(isDuplicate(spec(javaGame('offline'), 'operators'), entries, { name: 'Notch', id: NOTCH }), false);
  });

  test('ips compare case-insensitively', () => {
    const bans = [entry({ ip: '2001:DB8::1' })];
    const ipBans = spec(javaGame('running'), 'ip_bans');
    assert.equal(isDuplicate(ipBans, bans, { name: null, id: null, ip: '2001:db8::1' }), true);
    assert.equal(isDuplicate(ipBans, bans, { name: null, id: null, ip: '2001:db8::2' }), false);
  });
});

describe('banDate', () => {
  test('keeps the date of java timestamps and leaves other values alone', () => {
    assert.equal(banDate('2024-05-01 10:22:33 +0000'), '2024-05-01');
    assert.equal(banDate('2024-05-01T10:22:33Z'), '2024-05-01');
    assert.equal(banDate('forever'), 'forever');
    assert.equal(banDate('2024-05-01'), '2024-05-01');
  });
});
