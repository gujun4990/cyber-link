import assert from 'node:assert/strict';
import { test } from 'node:test';

import { applyStateRefresh } from './appState.js';

test('applyStateRefresh clears refresh failure state on success', () => {
  const current = {
    device: {
      room: '核心-01',
      pcId: '终端-05',
      ac: { isOn: false, temp: 18 },
      lightOn: false,
      acAvailable: false,
      lightAvailable: false,
      connected: false,
      initError: 'initial failure',
    },
    initFailed: true,
    actionFailed: false,
    refreshFailed: true,
    refreshError: 'refresh failed',
  };

  const snapshot = {
    room: '核心-01',
    pcId: '终端-05',
    ac: { isOn: true, temp: 24 },
    lightOn: true,
    acAvailable: true,
    lightAvailable: true,
    connected: true,
    initError: undefined,
  };

  const next = applyStateRefresh(current, snapshot);

  assert.equal(next.device, snapshot);
  assert.equal(next.initFailed, false);
  assert.equal(next.actionFailed, false);
  assert.equal(next.refreshFailed, false);
  assert.equal(next.refreshError, null);
});
