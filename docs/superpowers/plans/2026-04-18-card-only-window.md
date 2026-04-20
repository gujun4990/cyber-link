# Card-Only Window Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show only the center card UI with no visible outer shell, while preserving the card's original inner effects, layout, and useful comments.

**Architecture:** The Tauri layer will provide a frameless, transparent window with no system chrome, and the React layer will render only the card itself as the visible surface. The card should keep its original internal structure, motion, and comments; the only removal is the outer shell/overlay around it.

**Tech Stack:** Tauri 1, Rust, React, TypeScript, Tailwind CSS

---

### Task 1: Render only the card surface

**Files:**
- Modify: `src/App.tsx:181-420`
- Test: `src/shellRemoval.test.js`

- [ ] **Step 1: Add the regression assertion for card-only rendering**

```javascript
test('App root renders the card as the only visible surface', () => {
  const source = readFileSync(new URL('./App.tsx', import.meta.url), 'utf8');

  assert.equal(source.includes('fixed inset-0 m-auto'), true);
  assert.equal(source.includes('flex min-h-screen items-center justify-center p-4 overflow-hidden'), false);
  assert.equal(source.includes('fixed inset-0 pointer-events-none z-50'), false);
  assert.equal(source.includes('w-64 flex flex-col py-2'), true);
  assert.equal(source.includes('底部信号栏'), true);
  assert.equal(source.includes('currentTime.toLocaleTimeString'), true);
});
```

- [ ] **Step 2: Verify the test fails before applying the card-only structure**

Run: `node --test src/shellRemoval.test.js`
Expected: FAIL if the outer shell wrapper is still present.

- [ ] **Step 3: Remove the outer shell wrapper and keep the card centered on its own**

```tsx
return (
  <>
    <motion.div
      initial={{ opacity: 0, scale: 0.95, rotateX: 5 }}
      animate={{ opacity: 1, scale: 1, rotateX: 0 }}
      className="fixed inset-0 m-auto w-full max-w-[700px] aspect-[16/10] bg-[#0c2461]/90 backdrop-blur-3xl border-2 border-cyan-400/30 rounded-2xl overflow-visible flex flex-col shadow-[0_0_150px_rgba(6,182,212,0.4),inset_0_0_100px_rgba(0,0,0,0.5)]"
    >
      {/* preserve the original card internals here */}
    </motion.div>
  </>
);
```

- [ ] **Step 4: Preserve the card's internal effects and comments**

```tsx
// Keep the existing internal layers:
// - carbon texture
// - brushed metal
// - holographic gradient
// - corner decorations
// - top status bar
// - center temperature panel
// - side controls
// - bottom signal bar
```

- [ ] **Step 5: Re-run the React build check**

Run: `npm run build`
Expected: PASS.

### Task 2: Keep the frameless transparent window

**Files:**
- Modify: `src-tauri/tauri.conf.json:36-45`
- Test: `src/shellRemoval.test.js`

- [ ] **Step 1: Assert the window remains undecorated and transparent**

```javascript
test('tauri window is configured as a single transparent surface', () => {
  const tauriConfig = JSON.parse(
    readFileSync(new URL('../src-tauri/tauri.conf.json', import.meta.url), 'utf8'),
  );

  const [mainWindow] = tauriConfig.tauri.windows;

  assert.equal(mainWindow.label, 'main');
  assert.equal(mainWindow.visible, false);
  assert.equal(mainWindow.decorations, false);
  assert.equal(mainWindow.transparent, true);
});
```

- [ ] **Step 2: Keep the main window as the only runtime surface**

```json
{
  "tauri": {
    "windows": [
      {
        "label": "main",
        "title": "CyberLink",
        "visible": false,
        "decorations": false,
        "transparent": true,
        "center": true,
        "resizable": true
      }
    ]
  }
}
```

### Task 3: Verify runtime behavior

**Files:**
- Test: `src/shellRemoval.test.js`

- [ ] **Step 1: Run the full verification suite**

Run: `node --test src/shellRemoval.test.js`
Run: `cargo test`
Run: `cargo check`
Expected: PASS, with existing warnings only if present.
