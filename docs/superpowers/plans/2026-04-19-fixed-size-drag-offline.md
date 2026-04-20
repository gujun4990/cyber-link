# Fixed Size, Drag, and Offline State Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep the interface fixed-size, make the top bar draggable without double-click side effects, and ensure startup/offline renders the whole UI in an off state.

**Architecture:** Keep the existing Tauri single-window architecture and the shared `windowSize.json` as the single source of truth for size. Replace system drag-region behavior with explicit top-bar drag handling so double-click can be ignored, disable window resizing in Tauri, and derive all startup/offline visual state from a separate display flag so any effect that implies “active” stays off until a real HA snapshot is known.

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
  assert.equal(source.includes('appWindow.startDragging()'), true);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test src/appWindowBehavior.test.js`
Expected: FAIL because the top bar still uses system drag-region behavior.

- [ ] **Step 3: Write minimal implementation**

```tsx
const dragTopBar = async (event: React.MouseEvent<HTMLDivElement>) => {
  if (event.button !== 0 || event.target !== event.currentTarget) {
    return;
  }

  try {
    await appWindow.startDragging();
  } catch (error) {
    console.error('Failed to drag window', error);
  }
};

<div
  className="relative z-[70] flex items-center justify-between px-4 py-3 bg-black/40 border-b border-white/5 backdrop-blur-xl select-none"
  onMouseDown={(event) => {
    void dragTopBar(event);
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

### Task 2: Lock the window size

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Modify: `src/App.tsx`
- Modify: `src/appWindowBehavior.test.js`

- [ ] **Step 1: Write the failing test**

```javascript
test('tauri window is fixed size and not resizable', () => {
  const tauriConfig = JSON.parse(
    readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'),
  );

  const [mainWindow] = tauriConfig.tauri.windows;

  assert.equal(mainWindow.resizable, false);
  assert.equal(mainWindow.maximizable, false);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test src/appWindowBehavior.test.js`
Expected: FAIL because the window is still resizable in config.

- [ ] **Step 3: Write minimal implementation**

```json
{
  "label": "main",
  "title": "CyberControl HA Client",
  "visible": false,
  "decorations": false,
  "transparent": true,
  "center": true,
  "resizable": false,
  "maximizable": false
}
```

```tsx
<motion.div
  className="fixed inset-0 m-auto border-[1.5px] border-white/10 overflow-hidden flex flex-col ..."
  style={{
    width: windowSize.width,
    height: windowSize.height,
  }}
>
```

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test src/appWindowBehavior.test.js`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/tauri.conf.json src/App.tsx src/appWindowBehavior.test.js
git commit -m "fix: lock the window size"
```

### Task 3: Render startup and offline UI in the off state

**Files:**
- Modify: `src/App.tsx`
- Modify: `src/shellRemoval.test.js`

- [ ] **Step 1: Write the failing test**

```javascript
test('startup renders the full interface off until HA state is known', () => {
  const source = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.equal(source.includes('const acDisplayOn = hasLoadedState && device.connected && device.ac.isOn;'), true);
  assert.equal(source.includes('const lightDisplayOn = hasLoadedState && device.connected && device.lightOn;'), true);
  assert.equal(source.includes('const coolingModeActive = hasLoadedState && device.connected && device.ac.isOn && device.ac.temp < 20;'), true);
  assert.equal(source.includes('const heatingModeActive = hasLoadedState && device.connected && device.ac.isOn && device.ac.temp > 26;'), true);
  assert.equal(source.includes('const tempDisplayOn = hasLoadedState && device.connected && device.ac.isOn;'), true);
});
```

- [ ] **Step 2: Run test to verify it fails**

Run: `node --test src/shellRemoval.test.js`
Expected: FAIL because the display flags are not yet separated for all active visuals.

- [ ] **Step 3: Write minimal implementation**

```tsx
const acDisplayOn = hasLoadedState && device.connected && device.ac.isOn;
const lightDisplayOn = hasLoadedState && device.connected && device.lightOn;
const coolingModeActive = hasLoadedState && device.connected && device.ac.isOn && device.ac.temp < 20;
const heatingModeActive = hasLoadedState && device.connected && device.ac.isOn && device.ac.temp > 26;
const tempDisplayOn = hasLoadedState && device.connected && device.ac.isOn;
```

Apply these flags to the AC switch, light switch, cooling/heating badges, temperature glow, and temperature text so startup/offline always renders them off/gray.

- [ ] **Step 4: Run test to verify it passes**

Run: `node --test src/shellRemoval.test.js`
Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src/App.tsx src/shellRemoval.test.js
git commit -m "fix: render startup visuals in the off state"
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
