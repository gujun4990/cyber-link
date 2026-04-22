import assert from 'node:assert/strict';
import { test } from 'node:test';

import { buildDevLaunchConfig } from './devLauncher.ts';

test('defaults dev runtime to tauri and keeps vite args', () => {
  const config = buildDevLaunchConfig(['node', 'scripts/dev.ts']);

  assert.equal(config.runtimeMode, 'tauri');
  assert.deepEqual(config.viteArgs, ['--port=5173', '--host=0.0.0.0']);
});

test('extracts mock runtime from argv and strips it from vite args', () => {
  const config = buildDevLaunchConfig(['node', 'scripts/dev.ts', '--runtime=mock', '--debug']);

  assert.equal(config.runtimeMode, 'mock');
  assert.deepEqual(config.viteArgs, ['--port=5173', '--host=0.0.0.0', '--debug']);
});
