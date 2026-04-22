# Windows Shutdown Pending Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** When Windows is actually shutting down, the app should send a shutdown intent immediately and Home Assistant should wait 30 seconds before applying the final offline/shutdown action.

**Architecture:** Keep Windows/Tauri as the shutdown-intent source, but move the 30-second wait and cancel logic into Home Assistant. The app emits a lightweight `shutdown_pending` signal during `WM_QUERYENDSESSION`; HA starts a timer, cancels it if `startup_online` arrives again, and only performs the final shutdown sync if the timer expires while the machine is still offline.

**Tech Stack:** Rust, React, Tauri 1, TypeScript, Home Assistant automations/timer helpers

---

### Task 1: Add shutdown-pending HA request helpers

**Files:**
- Modify: `src-tauri/src/action.rs`
- Test: `src-tauri/src/action.rs`

- [ ] **Step 1: Write the failing test**

Add a focused test that proves a shutdown-pending request turns on the HA helper entity instead of performing the final shutdown sync immediately.

```rust
#[tokio::test]
async fn shutdown_pending_turns_on_pending_helper() {
    // fake server should see a POST to:
    // /api/services/input_boolean/turn_on
    // with body {"entity_id":"input_boolean.cyber_link_shutdown_pending"}
    // and no AC/light shutdown requests yet
}

#[tokio::test]
async fn startup_online_clears_shutdown_pending_helper() {
    // fake server should see a POST to:
    // /api/services/input_boolean/turn_off
    // with body {"entity_id":"input_boolean.cyber_link_shutdown_pending"}
    // before the normal startup-on requests
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test shutdown_pending`

Expected: FAIL because both helper requests do not exist yet.

- [ ] **Step 3: Write the minimal implementation**

Add a small helper in `action.rs` that sends the pending signal to the fixed HA helper entity, and update `send_startup_online` so it clears the pending helper before turning devices back on.

```rust
const SHUTDOWN_PENDING_ENTITY_ID: &str = "input_boolean.cyber_link_shutdown_pending";

pub(crate) async fn send_shutdown_pending(config: &AppConfig) -> Result<()> {
    send_entity_toggle_request(config, SHUTDOWN_PENDING_ENTITY_ID, true).await
}

pub(crate) async fn send_startup_online(config: &AppConfig) -> Result<()> {
    let _ = send_entity_toggle_request(config, SHUTDOWN_PENDING_ENTITY_ID, false).await;
    // existing startup online logic continues here
    // send PC online, then AC and light startup requests as before
    Ok(())
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test shutdown_pending_turns_on_pending_helper`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/action.rs
git commit -m "feat: add shutdown pending HA helper"
```

### Task 2: Route Windows shutdown hook through shutdown-pending

**Files:**
- Modify: `src-tauri/src/main.rs`
- Test: `src-tauri/src/main.rs`

- [ ] **Step 1: Write the failing test**

Extract the shutdown-notification behavior into a small helper that can be tested without a real HWND. The test should assert the callback is invoked and the helper returns the query-end-session result.

```rust
#[test]
fn shutdown_notification_invokes_sender_and_returns_query_result() {
    let mut called = false;
    let result = shutdown_notification_response(|| {
        called = true;
    });

    assert!(called);
    assert_eq!(result, query_end_session_result());
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test shutdown_notification_invokes_sender_and_returns_query_result`

Expected: FAIL because the helper does not exist yet.

- [ ] **Step 3: Write the minimal implementation**

Add a helper in `main.rs` that wraps the shutdown response and calls `send_shutdown_pending` from the existing Windows shutdown hook.

```rust
fn shutdown_notification_response(send: impl FnOnce()) -> LRESULT {
    send();
    query_end_session_result()
}

unsafe extern "system" fn main_window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if handle_windows_message_kind(msg) {
        if let Ok(config) = load_config() {
            return shutdown_notification_response(|| {
                let _ = tauri::async_runtime::block_on(crate::action::send_shutdown_pending(&config));
            });
        }
        return query_end_session_result();
    }
    // existing proc logic stays unchanged
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test shutdown_notification_invokes_sender_and_returns_query_result`

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add src-tauri/src/main.rs
git commit -m "feat: send shutdown pending on windows close"
```

### Task 3: Document the HA timer helper flow

**Files:**
- Add: `docs/ha-shutdown-pending-automation.md`
- Modify: `README.md`

- [ ] **Step 1: Write the documentation**

Document the HA entities and automations required for the 30-second delay.

```md
# HA Shutdown Pending Automation

- `input_boolean.cyber_link_shutdown_pending`
- `timer.cyber_link_shutdown_delay`

Automation flow:
1. `shutdown_pending` -> turn on pending boolean and start 30-second timer
2. `startup_online` -> cancel timer and clear pending boolean
3. timer finished while pending is on -> run the final shutdown sync
```

- [ ] **Step 2: Run a quick review pass**

Check that the doc names match the code constants and there are no placeholder sections.

- [ ] **Step 3: Update README**

Add a short pointer in the setup section so users know the shutdown-pending helper must be created in HA.

- [ ] **Step 4: Commit**

```bash
git add docs/ha-shutdown-pending-automation.md README.md
git commit -m "docs: add shutdown pending ha setup"
```

### Task 4: Full verification

**Files:**
- No code changes expected

- [ ] **Step 1: Run Rust tests**

Run: `cargo test`

Expected: PASS.

- [ ] **Step 2: Run frontend checks**

Run: `npm run lint`

Expected: PASS.

- [ ] **Step 3: Run the frontend build**

Run: `npm run build`

Expected: PASS.

- [ ] **Step 4: Commit if anything changed**

If verification surfaced fixes, commit those changes before finishing.
