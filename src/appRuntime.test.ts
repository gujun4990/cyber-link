import assert from 'node:assert/strict';
import { test } from 'node:test';

import { createAppRuntime } from './appRuntime.ts';

test('explicit mock mode uses mock runtime', () => {
  const runtime = createAppRuntime({ mode: 'mock' });

  assert.equal(runtime.mode, 'mock');
});

test('mock runtime starts from offline snapshot', async () => {
  const runtime = createAppRuntime({ mode: 'mock' });

  const initial = await runtime.initializeApp();

  assert.equal(initial.connected, false);
  assert.equal(initial.acAvailable, false);
  assert.equal(initial.switchAvailable, false);
  assert.equal(initial.mainLightAvailable, false);
  assert.equal(initial.doorSignLightAvailable, false);
  assert.equal(initial.ac.isOn, false);
  assert.equal(initial.switchOn, false);
  assert.equal(initial.mainLightOn, false);
  assert.equal(initial.doorSignLightOn, false);
});

test('mock runtime routes switch toggles by target', async () => {
  const runtime = createAppRuntime({ mode: 'mock' });
  const snapshots: Array<{ doorSignLightOn: boolean; mainLightOn: boolean; switchOn: boolean }> = [];

  const unlisten = await runtime.subscribeStateRefresh((snapshot) => {
    snapshots.push({
      doorSignLightOn: snapshot.doorSignLightOn,
      mainLightOn: snapshot.mainLightOn,
      switchOn: snapshot.switchOn,
    });
  });

  const initial = await runtime.initializeApp();
  assert.equal(initial.doorSignLightAvailable, false);
  assert.equal(initial.mainLightAvailable, false);
  assert.equal(initial.switchAvailable, false);
  assert.equal(initial.doorSignLightOn, false);

  const live = await runtime.refreshHaState();
  assert.equal(live.doorSignLightAvailable, true);
  assert.equal(live.mainLightAvailable, true);
  assert.equal(live.switchAvailable, true);
  assert.equal(live.doorSignLightOn, true);

  const afterToggle = await runtime.handleHaAction('switch_toggle', 'mainLight');
  assert.equal(afterToggle.mainLightOn, false);
  assert.equal(afterToggle.doorSignLightOn, true);

  const afterRefresh = await runtime.refreshHaState();
  assert.equal(afterRefresh.mainLightOn, false);

  unlisten();

  assert.ok(snapshots.length >= 2);
  assert.equal(snapshots[0]?.doorSignLightOn, true);
  assert.equal(snapshots.at(-1)?.mainLightOn, false);
});

test('mock runtime rejects switch toggles without a target', async () => {
  const runtime = createAppRuntime({ mode: 'mock' });

  await assert.rejects(
    () => runtime.handleHaAction('switch_toggle'),
    /missing light target/i,
  );
});

test('mock runtime applies temperature updates', async () => {
  const runtime = createAppRuntime({ mode: 'mock' });

  const initial = await runtime.initializeApp();
  assert.equal(initial.ac.isOn, false);

  const live = await runtime.refreshHaState();
  assert.equal(live.ac.temp, 24);

  const afterTemp = await runtime.handleHaAction('ac_set_temp', undefined, 26);
  assert.equal(afterTemp.ac.temp, 26);
  assert.equal(afterTemp.ac.isOn, true);
});

test('mock runtime keeps shutdown snapshots unchanged', async () => {
  const runtime = createAppRuntime({ mode: 'mock' });
  const initial = await runtime.initializeApp();
  const live = await runtime.refreshHaState();

  const afterShutdown = await runtime.handleHaAction('shutdown_signal');

  assert.deepEqual(initial, {
    ...initial,
    connected: false,
    acAvailable: false,
    switchAvailable: false,
    ambientLightAvailable: false,
    mainLightAvailable: false,
    doorSignLightAvailable: false,
  });
  assert.deepEqual(afterShutdown, live);
});

test('mock runtime exposes ambient light aliases', async () => {
  const runtime = createAppRuntime({ mode: 'mock' });
  const initial = await runtime.initializeApp();

  assert.equal(initial.ambientLightAvailable, false);

  const live = await runtime.refreshHaState();

  assert.equal(live.ambientLightAvailable, true);
  assert.equal(live.ambientLightOn, false);
});
