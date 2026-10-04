import assert from 'node:assert/strict';
import { describe, test } from 'node:test';
import {
  type Granted,
  kickAccess,
  listAccess,
  mutationMethod,
  onlineAccess,
  whitelistToggleAccess,
} from '../frontend/src/lib/access.ts';

const all: Granted = { console: true, readConsole: true, writeFiles: true };
const consoleOnly: Granted = { console: true, readConsole: true, writeFiles: false };
const filesOnly: Granted = { console: false, readConsole: false, writeFiles: true };

describe('mutationMethod', () => {
  test('java uses commands while running and files while offline', () => {
    assert.equal(mutationMethod('java', 'running'), 'command');
    assert.equal(mutationMethod('java', 'offline'), 'file');
  });

  test('bedrock always edits files', () => {
    assert.equal(mutationMethod('bedrock', 'running'), 'file');
    assert.equal(mutationMethod('bedrock', 'offline'), 'file');
  });

  test('nothing runs while starting or stopping', () => {
    for (const state of ['starting', 'stopping'] as const) {
      assert.equal(mutationMethod('java', state), null);
      assert.equal(mutationMethod('bedrock', state), null);
    }
  });
});

describe('listAccess', () => {
  test('java needs the console while running and file writes while offline', () => {
    assert.deepEqual(listAccess('java', 'running', consoleOnly), { visible: true, blocker: null });
    assert.deepEqual(listAccess('java', 'offline', consoleOnly), { visible: true, blocker: 'noFiles' });
    assert.deepEqual(listAccess('java', 'running', filesOnly), { visible: true, blocker: 'noConsole' });
    assert.deepEqual(listAccess('java', 'offline', filesOnly), { visible: true, blocker: null });
  });

  test('bedrock needs file writes always and the console for the reload while running', () => {
    assert.deepEqual(listAccess('bedrock', 'offline', filesOnly), { visible: true, blocker: null });
    assert.deepEqual(listAccess('bedrock', 'running', filesOnly), { visible: true, blocker: 'noConsole' });
    assert.equal(listAccess('bedrock', 'running', consoleOnly).visible, false);
  });

  test('transitions block even with every permission', () => {
    assert.deepEqual(listAccess('java', 'stopping', all), { visible: true, blocker: 'transition' });
  });

  test('hidden when no permission could ever apply', () => {
    const none: Granted = { console: false, readConsole: true, writeFiles: false };
    assert.equal(listAccess('java', 'offline', none).visible, false);
  });
});

describe('whitelistToggleAccess', () => {
  test('a running bedrock server only edits server.properties', () => {
    assert.deepEqual(whitelistToggleAccess('bedrock', 'running', filesOnly), { visible: true, blocker: null });
  });

  test('a running java server needs the console', () => {
    assert.deepEqual(whitelistToggleAccess('java', 'running', filesOnly), { visible: true, blocker: 'noConsole' });
    assert.deepEqual(whitelistToggleAccess('java', 'offline', filesOnly), { visible: true, blocker: null });
  });
});

describe('kick and online', () => {
  test('only while running', () => {
    assert.equal(kickAccess('running', all).blocker, null);
    assert.equal(kickAccess('offline', all).blocker, 'notRunning');
    assert.equal(onlineAccess('starting', all).blocker, 'transition');
    assert.equal(onlineAccess('offline', all).blocker, 'notRunning');
  });

  test('listing online players needs to send and read the console', () => {
    assert.equal(onlineAccess('running', { ...all, readConsole: false }).visible, false);
    assert.equal(kickAccess('running', filesOnly).visible, false);
  });
});
