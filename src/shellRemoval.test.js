import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

test('App root should not use a full-screen black shell container', () => {
  const source = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.doesNotMatch(source, /min-h-screen bg-\[#050c2d\]/);
  assert.match(source, /max-w-\[700px\] aspect-\[16\/10\]/);
});
