import assert from 'node:assert/strict';
import { describe, test } from 'node:test';
import {
  normalizeItemId,
  validateId,
  validateIp,
  validateItemId,
  validateName,
  validateReason,
} from '../frontend/src/lib/validation.ts';
import { bedrockGame, javaGame } from './fixtures.ts';

const java = javaGame('offline');
const bedrock = bedrockGame('offline');

describe('validateName', () => {
  test('checks the name pattern of the game descriptor', () => {
    for (const name of ['Notch', 'a', 'x_1234567890ABCD', '.BedrockGuy', '*Prefixed']) {
      assert.equal(validateName(java, name), null, name);
    }
    for (const name of ['x_1234567890ABCDE', 'with space', 'dash-name', '..double', 'ümlaut']) {
      assert.equal(validateName(java, name), 'name', name);
    }
  });

  test('another game brings its own pattern', () => {
    assert.equal(validateName(bedrock, 'Some Gamer 42'), null);
    assert.equal(validateName(bedrock, 'a'.repeat(32)), null);
    assert.equal(validateName(bedrock, 'a'.repeat(33)), 'name');
    assert.equal(validateName(bedrock, ' Leading'), 'name');
    assert.equal(validateName(bedrock, 'under_score'), 'name');
  });

  test('empty is required whatever the pattern accepts', () => {
    assert.equal(validateName({ player_name: { pattern: '^.*$' } }, ''), 'required');
  });
});

describe('validateId', () => {
  test('checks the id pattern of the game descriptor', () => {
    assert.equal(validateId(java, '069a79f4-44e9-4726-a5be-fca90e38aaf5'), null);
    assert.equal(validateId(java, '069A79F444E94726A5BEFCA90E38AAF5'), null);
    assert.equal(validateId(java, '069a79f4-44e94726a5befca90e38aaf5'), 'id');
    assert.equal(validateId(bedrock, '2535416409688276'), null);
    assert.equal(validateId(bedrock, '1'.repeat(21)), 'id');
    assert.equal(validateId(bedrock, '12a'), 'id');
  });

  test('an empty id is allowed', () => {
    assert.equal(validateId(java, ''), null);
    assert.equal(validateId(bedrock, ''), null);
  });
});

describe('validateReason', () => {
  test('counts characters, not UTF-16 units, up to 256', () => {
    assert.equal(validateReason('x'.repeat(256)), null);
    assert.equal(validateReason('x'.repeat(257)), 'reasonLength');
    // 256 astral characters are 512 UTF-16 code units but still fit
    assert.equal(validateReason('😀'.repeat(256)), null);
  });

  test('rejects control characters', () => {
    assert.equal(validateReason('griefing\nspawn'), 'reasonControl');
    assert.equal(validateReason('tab\there'), 'reasonControl');
    assert.equal(validateReason('Griefing at spawn, see #reports'), null);
  });
});

describe('validateIp', () => {
  test('ipv4 needs four in-range octets without leading zeros', () => {
    assert.equal(validateIp('192.168.0.1'), null);
    assert.equal(validateIp('0.0.0.0'), null);
    assert.equal(validateIp('256.1.1.1'), 'ip');
    assert.equal(validateIp('1.2.3'), 'ip');
    assert.equal(validateIp('01.2.3.4'), 'ip');
  });

  test('ipv6 accepts full, compressed and ipv4-mapped forms', () => {
    for (const ip of ['2001:db8:0:0:0:0:0:1', '2001:db8::1', '::1', '::', 'fe80::', '::ffff:192.0.2.1', '1:2:3:4:5:6:7::']) {
      assert.equal(validateIp(ip), null, ip);
    }
    for (const ip of ['1:2:3:4:5:6:7:8:9', '1::2::3', '12345::', '1:2:3:4:5:6:7:8::', '[::1]', '::1.2.3.4.5', '1.2.3.4::']) {
      assert.equal(validateIp(ip), 'ip', ip);
    }
  });

  test('empty is required', () => {
    assert.equal(validateIp(''), 'required');
  });
});

describe('item ids', () => {
  test('the namespace defaults to minecraft; surrounding spaces go', () => {
    assert.equal(normalizeItemId(' diamond '), 'minecraft:diamond');
    assert.equal(normalizeItemId('minecraft:oak_log'), 'minecraft:oak_log');
    assert.equal(normalizeItemId('create:brass_ingot'), 'create:brass_ingot');
    assert.equal(normalizeItemId('mod.name:tools/big-hammer'), 'mod.name:tools/big-hammer');
  });

  test('ids the give command would reject are invalid', () => {
    for (const id of ['Diamond', 'minecraft:diamond sword', 'a:b:c', 'minecraft:', ':stone', 'stone{nbt:1}']) {
      assert.equal(normalizeItemId(id), null, id);
      assert.equal(validateItemId(id), 'itemId', id);
    }
    assert.equal(validateItemId('  '), 'required');
    assert.equal(validateItemId('stone'), null);
  });
});
