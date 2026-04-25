import assert from 'node:assert/strict';
import { test } from 'node:test';

import { applyStateRefresh, shouldIgnoreRevertedActionRefresh } from './appState.js';

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

test('shouldIgnoreRevertedActionRefresh ignores stale reverted action refreshes', () => {
  const before = {
    room: '核心-01',
    pcId: '终端-05',
    ac: { isOn: true, temp: 29 },
    ambientLightOn: false,
    mainLightOn: false,
    doorSignLightOn: false,
    acAvailable: true,
    ambientLightAvailable: false,
    mainLightAvailable: false,
    doorSignLightAvailable: false,
    lightCount: 0,
    connected: true,
  };

  const actionState = {
    room: '核心-01',
    pcId: '终端-05',
    ac: { isOn: true, temp: 30 },
    ambientLightOn: false,
    mainLightOn: false,
    doorSignLightOn: false,
    acAvailable: true,
    ambientLightAvailable: false,
    mainLightAvailable: false,
    doorSignLightAvailable: false,
    lightCount: 0,
    connected: true,
  };

  const revertedState = {
    ...actionState,
    ac: { isOn: true, temp: 29 },
  };

  assert.equal(
    shouldIgnoreRevertedActionRefresh(before, revertedState, { action: 'ac_set_temp', value: 30 }),
    true,
  );

  assert.equal(
    shouldIgnoreRevertedActionRefresh(before, revertedState, { action: 'ac_toggle' }),
    true,
  );

  assert.equal(
    shouldIgnoreRevertedActionRefresh(before, { ...revertedState, ambientLightOn: false }, { action: 'switch_toggle', target: 'ambientLight' }),
    true,
  );
});

test('shouldIgnoreRevertedActionRefresh keeps refreshed ac metadata', () => {
  const before = {
    room: '核心-01',
    pcId: '终端-05',
    ac: {
      isOn: true,
      temp: 26,
      minTemp: 16,
      maxTemp: 30,
      targetTempStep: 1,
      temperatureUnit: '°C',
      unitOfMeasurement: '°C',
    },
    ambientLightOn: false,
    mainLightOn: false,
    doorSignLightOn: false,
    acAvailable: true,
    ambientLightAvailable: false,
    mainLightAvailable: false,
    doorSignLightAvailable: false,
    lightCount: 0,
    connected: true,
  };

  const next = {
    ...before,
    ac: {
      ...before.ac,
      temp: 26,
      maxTemp: 28,
    },
  };

  assert.equal(
    shouldIgnoreRevertedActionRefresh(before, next, { action: 'ac_set_temp', value: 26 }),
    false,
  );
});
