# Tray Double-Click Open Window Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Double-clicking the system tray icon should reopen the main app window and bring it to the foreground.

**Architecture:** Reuse the existing `show_main_window` helper in the Windows Tauri entrypoint so tray activation and tray-menu open both use the same window-restoration path. Keep the current single-instance behavior and close-to-tray behavior unchanged.

**Tech Stack:** Tauri 1, Rust, Windows system tray APIs, Node test runner

---

### Task 1: Wire tray double-click to the existing open-window path

**Files:**
- Modify: `src-tauri/src/main.rs:731-753, 1052-1063`
- Test: `src/shellRemoval.test.js`

- [ ] **Step 1: Add a regression assertion for tray double-click handling**

```javascript
test('windows tray double click reopens the main window', () => {
  const mainSource = readFileSync(new URL('../src-tauri/src/main.rs', import.meta.url), 'utf8');

  assert.match(mainSource, /show_main_window\(app\)/);
  assert.match(mainSource, /SystemTrayEvent::DoubleClick \{ \.\.\. \} => show_main_window\(app\),/);
  assert.match(mainSource, /\.on_system_tray_event\(handle_tray_event\)/);
});
```

- [ ] **Step 2: Verify the new test fails before the code change**

Run: `node --test src/shellRemoval.test.js`
Expected: FAIL because `SystemTrayEvent::DoubleClick` is not yet handled.

- [ ] **Step 3: Route tray double-click events to `show_main_window`**

```rust
fn handle_tray_event(app: &AppHandle, event: SystemTrayEvent) {
    match event {
        SystemTrayEvent::MenuItemClick { id, .. } => match id.as_str() {
            TRAY_OPEN_ID => show_main_window(app),
            TRAY_EXIT_ID => app.exit(0),
            _ => {}
        },
        SystemTrayEvent::DoubleClick { .. } => show_main_window(app),
        _ => {}
    }
}
```

- [ ] **Step 4: Re-run the regression test**

Run: `node --test src/shellRemoval.test.js`
Expected: PASS.

- [ ] **Step 5: Run the full frontend and unit test suite**

Run: `node --test src/*.test.js`
Run: `npm run build`
Expected: PASS.
