import assert from 'node:assert/strict';
import { test } from 'node:test';

import { buildLightingCards } from './lightingCards.ts';

test('buildLightingCards returns only configured lights', () => {
  const cards = buildLightingCards({
    ambientLightAvailable: false,
    mainLightAvailable: false,
    doorSignLightAvailable: true,
    ambientLightOn: false,
    mainLightOn: false,
    doorSignLightOn: true,
  });

  assert.deepEqual(cards.map((card) => card.kind), ['doorSignLight']);
  assert.deepEqual(cards.map((card) => card.label), ['门牌灯']);
  assert.deepEqual(cards.map((card) => card.active), [true]);
});

test('buildLightingCards keeps priority order for multiple configured lights', () => {
  const cards = buildLightingCards({
    ambientLightAvailable: true,
    mainLightAvailable: false,
    doorSignLightAvailable: true,
    ambientLightOn: true,
    mainLightOn: false,
    doorSignLightOn: false,
  });

  assert.deepEqual(cards.map((card) => card.kind), ['ambientLight', 'doorSignLight']);
  assert.deepEqual(cards.map((card) => card.label), ['氛围灯', '门牌灯']);
  assert.deepEqual(cards.map((card) => card.active), [true, false]);
});
