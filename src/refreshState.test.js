import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

test('refresh button calls backend state refresh command', () => {
  const appSource = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.match(appSource, /invoke<DeviceState>\('refresh_ha_state'\)/);
  assert.doesNotMatch(appSource, /window\.location\.reload\(\)/);
});

test('backend exposes a dedicated ha refresh command', () => {
  const backendSource = readFileSync(
    new URL('../src-tauri/src/main.rs', import.meta.url),
    'utf8',
  );

  assert.match(backendSource, /refresh_ha_state/);
});
