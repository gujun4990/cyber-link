import assert from 'node:assert/strict';
import { test } from 'node:test';

import { shouldPauseUi, shouldRefreshOnResume } from './windowPause.ts';

test('window pause follows hidden state only', () => {
  assert.equal(shouldPauseUi(true), true);
  assert.equal(shouldPauseUi(false), false);
});

test('resume refresh only fires after a pause transition', () => {
  assert.equal(shouldRefreshOnResume({ wasPaused: true, isPaused: false, hasLoadedState: true }), true);
  assert.equal(shouldRefreshOnResume({ wasPaused: true, isPaused: true, hasLoadedState: true }), false);
  assert.equal(shouldRefreshOnResume({ wasPaused: false, isPaused: false, hasLoadedState: true }), false);
  assert.equal(shouldRefreshOnResume({ wasPaused: true, isPaused: false, hasLoadedState: false }), false);
});
