import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

test('refresh button uses disabled and spinning icon while refreshing', () => {
  const appSource = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.match(appSource, /const \[refreshing, setRefreshing\] = useState\(false\);/);
  assert.match(appSource, /disabled=\{refreshing\}/);
  assert.match(appSource, /animate-spin/);
});
