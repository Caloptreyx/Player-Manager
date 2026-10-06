import assert from 'node:assert/strict';
import { describe, test } from 'node:test';
import { capabilityAccess, editHint, gamePermissions } from '../frontend/src/lib/access.ts';
import { bedrockGame, javaGame } from './fixtures.ts';

const all = new Set(['control.console', 'control.read-console', 'files.create']);
const consoleOnly = new Set(['control.console', 'control.read-console']);
const filesOnly = new Set(['files.create']);

describe('capabilityAccess', () => {
  test('java edits need the console while running and file writes while offline', () => {
    assert.deepEqual(capabilityAccess(javaGame('running').edit, consoleOnly), { visible: true, blocker: null });
    assert.deepEqual(capabilityAccess(javaGame('offline').edit, consoleOnly), { visible: true, blocker: 'noFiles' });
    assert.deepEqual(capabilityAccess(javaGame('running').edit, filesOnly), { visible: true, blocker: 'noConsole' });
    assert.deepEqual(capabilityAccess(javaGame('offline').edit, filesOnly), { visible: true, blocker: null });
  });

  test('bedrock edits report the first missing permission and hide without file writes', () => {
    assert.deepEqual(capabilityAccess(bedrockGame('offline').edit, filesOnly), { visible: true, blocker: null });
    assert.deepEqual(capabilityAccess(bedrockGame('running').edit, filesOnly), { visible: true, blocker: 'noConsole' });
    assert.equal(capabilityAccess(bedrockGame('running').edit, consoleOnly).visible, false);
    assert.equal(capabilityAccess(bedrockGame('running').edit, new Set()).blocker, 'noFiles');
  });

  test('the server state blocks before any permission', () => {
    assert.deepEqual(capabilityAccess(javaGame('starting').edit, all), { visible: true, blocker: 'transition' });
    assert.deepEqual(capabilityAccess(javaGame('starting').edit, new Set()), { visible: false, blocker: 'transition' });
    assert.equal(capabilityAccess(javaGame('offline').kick, all).blocker, 'notRunning');
    assert.equal(capabilityAccess(javaGame('offline').online, all).blocker, 'notRunning');
    assert.equal(capabilityAccess(javaGame('starting').online, all).blocker, 'transition');
  });

  test('visible with any of visible_with, blocked by whichever required permission is missing', () => {
    const live = javaGame('running').profiles?.edit_live ?? null;
    assert.deepEqual(capabilityAccess(live, consoleOnly), { visible: true, blocker: null });
    assert.deepEqual(capabilityAccess(live, filesOnly), { visible: false, blocker: 'noConsole' });
    assert.equal(capabilityAccess(javaGame('running').kick, filesOnly).visible, false);
  });

  test('an empty visible_with needs nothing beyond the page permission', () => {
    assert.deepEqual(capabilityAccess(javaGame('running').online, new Set()), { visible: true, blocker: null });
    assert.deepEqual(capabilityAccess(bedrockGame('offline').online, new Set()), {
      visible: true,
      blocker: 'notRunning',
    });
  });

  test('permissions without a dedicated text fall back to the generic blocker', () => {
    const capability = { requires: ['files.update'], visible_with: ['files.update'], blocked: null };
    assert.deepEqual(capabilityAccess(capability, new Set()), { visible: false, blocker: 'noPermission' });
  });

  test('a capability the game lacks is hidden', () => {
    assert.deepEqual(capabilityAccess(null, all), { visible: false, blocker: null });
  });
});

describe('gamePermissions', () => {
  test('include the permissions of the profile capabilities', () => {
    assert.deepEqual(gamePermissions(javaGame('running')).sort(), [
      'control.console',
      'files.create',
      'files.read-content',
    ]);
    assert.deepEqual(gamePermissions(bedrockGame('running')).sort(), ['control.console', 'files.create']);
  });
});

describe('editHint', () => {
  test('follows the method, telling a file edit with a console reload apart', () => {
    assert.equal(editHint(javaGame('running').edit), 'command');
    assert.equal(editHint(javaGame('offline').edit), 'file');
    assert.equal(editHint(bedrockGame('running').edit), 'fileReload');
    assert.equal(editHint(bedrockGame('offline').edit), 'file');
  });

  test('blocked edits explain the state; no edits, no hint', () => {
    assert.equal(editHint(javaGame('starting').edit), 'transition');
    assert.equal(editHint({ requires: [], visible_with: [], blocked: 'not_running', method: 'command' }), 'notRunning');
    assert.equal(editHint(null), null);
  });
});
