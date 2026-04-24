import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

test('restoring the window restarts the animation subtree', () => {
  const source = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.match(source, /const \[animationEpoch, setAnimationEpoch\] = useState\(0\);/);
  assert.match(source, /setAnimationEpoch\(\(epoch\) => epoch \+ 1\);/);
  assert.match(source, /key=\{animationEpoch\}/);
});
