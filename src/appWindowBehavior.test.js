import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

test('app hides the native window from minimize and close controls', () => {
  const appSource = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.match(appSource, /import \{ appWindow \} from '@tauri-apps\/api\/window';/);
  assert.match(appSource, /await appWindow\.hide\(\);/);
  assert.equal(appSource.includes('layoutId="tray-icon"'), false);
  assert.equal(appSource.includes('setIsMinimized'), false);
});
