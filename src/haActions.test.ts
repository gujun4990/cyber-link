import assert from 'node:assert/strict';
import { test } from 'node:test';
import { clampTemp, buildActionPayload } from './haActions.ts';

test('clamps temperature changes to the supported range', () => {
  assert.equal(clampTemp(16, -1), 16);
  assert.equal(clampTemp(29, 5), 30);
  assert.equal(clampTemp(22, 1), 23);
});

test('builds startup and shutdown payloads', () => {
  assert.deepEqual(buildActionPayload('startup'), { action: 'startup_online' });
  assert.deepEqual(buildActionPayload('shutdown'), { action: 'shutdown_signal' });
});
