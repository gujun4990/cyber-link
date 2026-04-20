# Remove Window Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** show only the centered main control surface from `w.jpeg` and remove the full-window black shell appearance from `x.png`, while preserving the motion/animation inside the main surface.

**Architecture:** Keep the single Tauri `main` window, but simplify the React root layout so the visible content is the centered control card rather than a full-screen dark shell. Preserve the internal animated effects inside the card and keep all startup and refresh logic unchanged.

**Tech Stack:** React, motion/react, Tailwind CSS, Tauri v1

---

### Task 1: Add Shell Removal Regression Tests

**Files:**
- Modify: `src/App.tsx`
- Test: `src/shellRemoval.test.js`

- [ ] **Step 1: Write the failing test**

```javascript
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';

test('App root should not use a full-screen black shell container', () => {
  const source = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.doesNotMatch(source, /min-h-screen bg-\[#050c2d\]/);
  assert.match(source, /max-w-\[700px\] aspect-\[16\/10\]/);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test src/shellRemoval.test.js`
Expected: FAIL because the root container still uses the full-screen black shell classes.

### Task 2: Remove the Window Shell Layout

**Files:**
- Modify: `src/App.tsx`
- Test: `src/shellRemoval.test.js`

- [ ] **Step 1: Implement the minimal layout change**

```tsx
return (
  <div className="min-h-screen flex items-center justify-center p-4">
    <AnimatePresence>
      {!isMinimized && (
        <motion.div className="relative w-full max-w-[700px] aspect-[16/10] ...">
          ...
        </motion.div>
      )}
    </AnimatePresence>
  </div>
);
```

The root container should stop painting a full-window dark shell; the inner card remains unchanged so its animation and styling stay visible.

- [ ] **Step 2: Run the test to verify it passes**

Run: `node --test src/shellRemoval.test.js`
Expected: PASS.

### Task 3: Verify UI And App Checks

**Files:**
- Test: `src/App.tsx`
- Test: `src/shellRemoval.test.js`

- [ ] **Step 1: Run the frontend build and typecheck**

Run: `npm run build && npm run lint`
Expected: successful build and no TypeScript errors.

- [ ] **Step 2: Run the Rust suite unchanged**

Run: `cargo test`
Expected: all existing Rust tests still pass because only the React root layout changed.

- [ ] **Step 3: Commit the implementation**

```bash
git add src/App.tsx src/shellRemoval.test.js
git commit -m "fix: remove window shell from main surface"
```
