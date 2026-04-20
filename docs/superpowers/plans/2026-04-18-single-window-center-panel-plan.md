# Single Window Center Panel Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove the black background window and leave only the centered main panel visible.

**Architecture:** The app should present one visible Tauri window and one React surface. The Tauri layer will own the single window lifecycle and startup visibility, while the React layer will shrink its footprint to only the center panel instead of painting a full-screen shell. The end result should be a single centered panel with no extra black backdrop or secondary shell.

**Tech Stack:** Rust, Tauri 1, React, TypeScript, Tailwind CSS

---

### Task 1: Confirm the single visible window path

**Files:**
- Modify: `src-tauri/src/main.rs:677-1053`
- Test: `src-tauri/src/main.rs` unit tests

- [ ] **Step 1: Add a failing test for the startup window path**

```rust
#[test]
fn startup_path_returns_single_visible_main_window() {
    assert_eq!(startup_window_action(StartupMode::Manual), StartupWindowAction::Show);
    assert_eq!(startup_window_action(StartupMode::Autostart), StartupWindowAction::Hide);
}
```

- [ ] **Step 2: Run the targeted tests to verify the current behavior is still only described, not duplicated**

Run: `cargo test startup_path_returns_single_visible_main_window -- --nocapture`
Expected: PASS after the existing startup action logic remains unchanged.

- [ ] **Step 3: Keep the startup logic on one Tauri window and make the single-instance restore path return early before any UI shell creation**

```rust
pub fn run() {
    if try_restore_existing_main_window() {
        return;
    }

    let startup_mode = startup_mode_from_args(std::env::args());
    tauri::Builder::default()
        .manage(SharedState(Mutex::new(initial_snapshot())))
        .setup(move |app| {
            if let Some(window) = app.get_window("main") {
                if let Err(err) = install_shutdown_hook(&window) {
                    eprintln!("failed to install shutdown hook: {err}");
                }
            }
            match startup_mode {
                StartupMode::Autostart => {
                    if let Some(window) = app.get_window("main") {
                        let _ = window.hide();
                    }
                }
                StartupMode::Manual => {
                    if let Some(window) = app.get_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            initialize_app,
            refresh_ha_state,
            handle_ha_action,
            set_autostart_enabled
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
```

- [ ] **Step 4: Re-run the Rust test suite**

Run: `cargo test`
Expected: PASS with no regressions.

### Task 2: Shrink the React surface to the center panel only

**Files:**
- Modify: `src/App.tsx:181-486`
- Test: `src/App.tsx` behavior via existing build/test flow

- [ ] **Step 1: Add a regression test for the app shell being removed**

```tsx
// Pseudocode for the existing test setup:
// render(<App />)
// expect(screen.getByText('空调控制系统')).toBeVisible()
// expect(screen.queryByRole('banner')).toBeNull()
```

- [ ] **Step 2: Run the app UI tests or build check to confirm the shell test fails before the layout change**

Run: `npm test` or the repo's existing frontend test command
Expected: The shell-related assertion fails before the layout change.

- [ ] **Step 3: Replace the full-screen wrapper with a centered panel-only container**

```tsx
return (
  <div className="min-h-screen flex items-center justify-center p-4 overflow-hidden bg-transparent">
    <motion.div
      initial={{ opacity: 0, scale: 0.95, rotateX: 5 }}
      animate={{ opacity: 1, scale: 1, rotateX: 0 }}
      className="relative w-full max-w-[700px] aspect-[16/10] bg-[#0c2461]/90 backdrop-blur-3xl border-2 border-cyan-400/30 rounded-2xl overflow-visible flex flex-col shadow-[0_0_150px_rgba(6,182,212,0.4),inset_0_0_100px_rgba(0,0,0,0.5)]"
    >
      {/* existing panel contents stay here */}
    </motion.div>
  </div>
);
```

- [ ] **Step 4: Remove any remaining outer shell styling that paints a black full-window backdrop**

```tsx
// Keep the centered panel and its internal background layers.
// Remove any outer wrapper styles that create a separate dark shell.
```

- [ ] **Step 5: Re-run the frontend verification command**

Run: `npm test`
Expected: PASS, and the app renders only the center panel region.

### Task 3: Verify the window/background pair on Windows packaging

**Files:**
- Modify: `src-tauri/tauri.conf.json:1-45` only if the single window needs decoration/transparent tweaks
- Test: GitHub Actions Windows build and local `cargo test`

- [ ] **Step 1: Check whether the visible black window is the Tauri window background or a separate shell layer**

```rust
// Use the existing single-window setup and inspect whether the frontend still paints a full-screen dark backdrop.
```

- [ ] **Step 2: If needed, set the main window to a transparent, borderless presentation in `tauri.conf.json`**

```json
{
  "tauri": {
    "windows": [
      {
        "label": "main",
        "title": "CyberControl HA Client",
        "visible": false,
        "decorations": false,
        "transparent": true,
        "resizable": true
      }
    ]
  }
}
```

- [ ] **Step 3: Re-run the full validation set**

Run: `cargo test`
Run: `npm test`
Expected: All pass.
