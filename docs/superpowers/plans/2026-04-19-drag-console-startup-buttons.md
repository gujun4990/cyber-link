# Drag, Console, and Startup Buttons Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make the top bar draggable without double-click side effects, remove the extra Windows console window, and ensure the AC/light buttons start in the off state until real HA state is known.

**Architecture:** Keep the existing single-window Tauri app. Replace system drag-region behavior with explicit top-bar drag handling so double-click can be ignored, mark the Windows binary as a GUI app so no console host appears, and separate button display state from backend truth so startup/loading/offline always renders both switches off until a real snapshot arrives.

**Tech Stack:** React, Tauri 1, Rust, TypeScript, Node test runner

---

### Task 1: Make the top bar drag-only

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/appWindowBehavior.test.js`

- [ ] **Step 1: Write the failing test**

```javascript
test('top bar supports drag but ignores double click', () => {
  const source = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.equal(source.includes('data-tauri-drag-region'), false);
  assert.equal(source.includes('onDoubleClickCapture'), true);
  assert.equal(source.includes('preventDefault()'), true);
  assert.equal(source.includes('stopPropagation()'), true);
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test src/appWindowBehavior.test.js`
Expected: FAIL because the top bar still uses system drag-region behavior.

- [ ] **Step 3: Write minimal implementation**

```tsx
import { appWindow } from '@tauri-apps/api/window';

const startWindowDrag = async (event: React.MouseEvent<HTMLDivElement>) => {
  if (event.button !== 0) return;
  if (event.target !== event.currentTarget) return;
  await appWindow.startDragging();
};

<div
  className="relative z-[70] flex items-center justify-between px-4 py-3 bg-black/40 border-b border-white/5 backdrop-blur-xl select-none"
  onMouseDown={(event) => {
    void startWindowDrag(event);
  }}
  onDoubleClickCapture={(event) => {
    event.preventDefault();
    event.stopPropagation();
  }}
>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test src/appWindowBehavior.test.js`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/App.tsx src/appWindowBehavior.test.js
git commit -m "fix: make the top bar drag-only"
```

### Task 2: Remove the Windows console window

**Files:**
- Modify: `src-tauri/src/main.rs`
- Modify: `src/shellRemoval.test.js`

- [ ] **Step 1: Write the failing test**

```javascript
test('windows binary is built without a console window', () => {
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.equal(mainSource.includes('#![cfg_attr(windows, windows_subsystem = "windows")]'), true);
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test src/shellRemoval.test.js`
Expected: FAIL because the subsystem attribute is missing.

- [ ] **Step 3: Write minimal implementation**

```rust
#![cfg_attr(windows, windows_subsystem = "windows")]

use anyhow::{anyhow, Result};
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test src/shellRemoval.test.js`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/main.rs src/shellRemoval.test.js
git commit -m "fix: remove the Windows console window"
```

### Task 3: Force startup switches off until HA state is known

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/shellRemoval.test.js`

- [ ] **Step 1: Write the failing test**

```javascript
test('startup renders both switches off until state is known', () => {
  const source = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.equal(source.includes('const acDisplayOn = hasLoadedState && device.connected && device.ac.isOn;'), true);
  assert.equal(source.includes('const lightDisplayOn = hasLoadedState && device.connected && device.lightOn;'), true);
  assert.equal(source.includes('active={acDisplayOn}'), true);
  assert.equal(source.includes('active={lightDisplayOn}'), true);
})
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test src/shellRemoval.test.js`
Expected: FAIL because the code still uses `device.ac.isOn` and `device.lightOn` directly for display state.

- [ ] **Step 3: Write minimal implementation**

```tsx
const acDisplayOn = hasLoadedState && device.connected && device.ac.isOn;
const lightDisplayOn = hasLoadedState && device.connected && device.lightOn;

<TechToggle active={acDisplayOn} ... />
<TechToggle active={lightDisplayOn} ... />
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test src/shellRemoval.test.js`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/App.tsx src/shellRemoval.test.js
git commit -m "fix: render startup switches off until state is loaded"
```

### Task 4: Verify the whole app still builds

**Files:**
- None

- [ ] **Step 1: Run the full JavaScript test suite**

Run: `node --test src/*.test.js`
Expected: PASS.

- [ ] **Step 2: Run the frontend build**

Run: `npm run build`
Expected: PASS.

- [ ] **Step 3: Run the Rust check**

Run: `cargo check`
Expected: PASS.
