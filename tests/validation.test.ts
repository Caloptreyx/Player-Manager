import assert from 'node:assert/strict';
import { describe, test } from 'node:test';
import { validateId, validateIp, validateName, validateReason } from '../frontend/src/lib/validation.ts';

describe('validateName', () => {
  test('java names are 1-16 word characters with an optional floodgate prefix', () => {
    for (const name of ['Notch', 'a', 'x_1234567890ABCD', '.BedrockGuy', '*Prefixed']) {
      assert.equal(validateName('java', name), null, name);
    }
    for (const name of ['x_1234567890ABCDE', 'with space', 'dash-name', '..double', 'ümlaut']) {
      assert.equal(validateName('java', name), 'javaName', name);
    }
    assert.equal(validateName('java', ''), 'required');
  });

  test('bedrock gamertags allow inner spaces up to 32 characters', () => {
    assert.equal(validateName('bedrock', 'Some Gamer 42'), null);
    assert.equal(validateName('bedrock', 'a'.repeat(32)), null);
    assert.equal(validateName('bedrock', 'a'.repeat(33)), 'bedrockName');
    assert.equal(validateName('bedrock', ' Leading'), 'bedrockName');
    assert.equal(validateName('bedrock', 'Trailing '), 'bedrockName');
    assert.equal(validateName('bedrock', 'under_score'), 'bedrockName');
  });
});

describe('validateId', () => {
  test('java accepts dashed and undashed uuids in any case', () => {
    assert.equal(validateId('java', '069a79f4-44e9-4726-a5be-fca90e38aaf5'), null);
    assert.equal(validateId('java', '069A79F444E94726A5BEFCA90E38AAF5'), null);
    assert.equal(validateId('java', '069a79f4-44e94726a5befca90e38aaf5'), 'uuid');
    assert.equal(validateId('java', '069a79f4'), 'uuid');
  });

  test('bedrock xuids are 1-20 digits', () => {
    assert.equal(validateId('bedrock', '2535416409688276'), null);
    assert.equal(validateId('bedrock', '1'.repeat(20)), null);
    assert.equal(validateId('bedrock', '1'.repeat(21)), 'xuid');
    assert.equal(validateId('bedrock', '12a'), 'xuid');
  });

  test('an empty id is allowed', () => {
    assert.equal(validateId('java', ''), null);
    assert.equal(validateId('bedrock', ''), null);
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
