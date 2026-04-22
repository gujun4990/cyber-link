import assert from 'node:assert/strict';
import { test } from 'node:test';

import { parseRuntimeModeFromArgs } from './runtimeMode.ts';

test('defaults to tauri mode', () => {
  assert.equal(parseRuntimeModeFromArgs(['npm', 'run', 'dev']), 'tauri');
});

test('parses mock mode from runtime arg', () => {
  assert.equal(parseRuntimeModeFromArgs(['npm', 'run', 'dev', '--', '--runtime=mock']), 'mock');
});

test('parses tauri mode from runtime arg', () => {
  assert.equal(parseRuntimeModeFromArgs(['npm', 'run', 'dev', '--', '--runtime=tauri']), 'tauri');
});
