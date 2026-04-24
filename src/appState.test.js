import assert from 'node:assert/strict';
import { test } from 'node:test';

import { applyStateRefresh } from './appState.js';

test('applyStateRefresh clears refresh failure state on success', () => {
  const current = {
    device: {
      room: '核心-01',
      pcId: '终端-05',
      ac: { isOn: false, temp: 18 },
      ambientLightOn: false,
      mainLightOn: false,
      doorSignLightOn: false,
      acAvailable: false,
      ambientLightAvailable: false,
      mainLightAvailable: false,
      doorSignLightAvailable: false,
      lightCount: 3,
      connected: false,
      initError: 'initial failure',
    },
    initFailed: true,
    actionFailed: true,
    refreshFailed: true,
    refreshError: 'refresh failed',
  };

  const snapshot = {
    room: '核心-01',
    pcId: '终端-05',
    ac: { isOn: true, temp: 24 },
    ambientLightOn: true,
    mainLightOn: true,
    doorSignLightOn: false,
    acAvailable: true,
    ambientLightAvailable: true,
    mainLightAvailable: true,
    doorSignLightAvailable: false,
    lightCount: 2,
    connected: true,
    initError: undefined,
  };

  const next = applyStateRefresh(current, snapshot);

  assert.equal(next.device, snapshot);
  assert.equal(next.initFailed, false);
  assert.equal(next.actionFailed, true);
  assert.equal(next.refreshFailed, false);
  assert.equal(next.refreshError, null);
});
