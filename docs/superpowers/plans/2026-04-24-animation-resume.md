# Animation Resume Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Re-trigger the dashboard animations when the window becomes visible again after being hidden or minimized.

**Architecture:** Keep the existing pause-on-hide behavior. Add a small resume token in `App.tsx` and advance it when the UI transitions from paused to visible so the animated dashboard subtree remounts and replays its motion effects once.

**Tech Stack:** React, TypeScript, Motion, Tauri

---

### Task 1: Add a resume restart signal

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/windowPause.ts`
- Test: `src/animationResume.test.js`

- [ ] **Step 1: Write the failing test**

```js
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

test('restoring the window restarts the animation subtree', () => {
  const source = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.match(source, /const \[animationEpoch, setAnimationEpoch\] = useState\(0\);/);
  assert.match(source, /setAnimationEpoch\(\(epoch\) => epoch \+ 1\);/);
  assert.match(source, /key=\{animationEpoch\}/);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test src/animationResume.test.js`
Expected: FAIL because `animationEpoch` is not present yet.

- [ ] **Step 3: Write minimal implementation**

```ts
const [animationEpoch, setAnimationEpoch] = useState(0);

useEffect(() => {
  if (shouldRefreshOnResume({ wasPaused: wasPausedRef.current, isPaused: uiPaused, hasLoadedState })) {
    void refreshHaState();
    setAnimationEpoch((epoch) => epoch + 1);
  }

  wasPausedRef.current = uiPaused;
}, [hasLoadedState, refreshHaState, uiPaused]);

<div key={animationEpoch} className="relative flex-1 flex flex-col overflow-hidden">
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test src/animationResume.test.js`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/App.tsx src/windowPause.ts src/animationResume.test.js docs/superpowers/plans/2026-04-24-animation-resume.md
git commit -m "fix: replay animations after window restore"
```
