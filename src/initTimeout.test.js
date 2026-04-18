import assert from 'node:assert/strict';
import { test } from 'node:test';
import { withTimeout } from './initTimeout.js';

test('withTimeout rejects when the operation stalls', async () => {
  await assert.rejects(
    withTimeout(new Promise(() => {}), 10, 'initialize_app timed out'),
    /initialize_app timed out/,
  );
});
