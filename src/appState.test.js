import assert from 'node:assert/strict';
import { test } from 'node:test';
import { applyStateRefresh } from './appState.js';

test('state refresh clears initialization failure', () => {
  const next = applyStateRefresh(
    {
      initFailed: true,
      actionFailed: true,
      device: {
        room: '核心-01',
        pcId: '终端-05',
        ac: { isOn: true, temp: 16 },
        lightOn: true,
        connected: false,
      },
    },
    {
      room: '核心-01',
      pcId: '终端-05',
      ac: { isOn: false, temp: 24 },
      lightOn: false,
      connected: true,
    },
  );

  assert.equal(next.initFailed, false);
  assert.equal(next.actionFailed, false);
  assert.deepEqual(next.device, {
    room: '核心-01',
    pcId: '终端-05',
    ac: { isOn: false, temp: 24 },
    lightOn: false,
    connected: true,
  });
});
