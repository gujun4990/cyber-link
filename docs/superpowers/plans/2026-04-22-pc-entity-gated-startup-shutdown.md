# PC Entity Gated Startup/Shutdown Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `pc_entity_id` control whether startup and shutdown only report machine state or also directly control AC and lights.

**Architecture:** When `pc_entity_id` is configured, the Rust startup and shutdown actions become state-only: startup sends the PC online notification and shutdown sends the PC offline notification, with no direct device toggles. When `pc_entity_id` is missing, the current direct-control fallback remains unchanged so existing single-node setups continue to work. The frontend does not change behavior; this is purely backend request routing and documentation.

**Tech Stack:** Rust, Tauri v1, Cargo tests, Markdown docs

---

### Task 1: Gate startup behavior on `pc_entity_id`

**Files:**
- Modify: `src-tauri/src/action.rs`
- Test: `src-tauri/src/action.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn startup_with_pc_entity_sends_only_pc_online() {
    // expect a POST to /api/services/input_boolean/turn_on
    // with body {"entity_id":"input_boolean.pc_05_online"}
    // and no AC/light turn_on requests
}

#[tokio::test]
async fn startup_without_pc_entity_keeps_direct_control() {
    // expect the existing AC/light startup requests
    // and no PC online request
}
```

- [ ] **Step 2: Run the tests and verify they fail**

Run: `cargo test startup_with_pc_entity_sends_only_pc_online`
Run: `cargo test startup_without_pc_entity_keeps_direct_control`

Expected: FAIL because `send_startup_online` still routes startup as a mixed PC/device action.

- [ ] **Step 3: Implement the minimal code**

```rust
pub(crate) async fn send_startup_online(config: &AppConfig) -> Result<()> {
    if config.pc_entity_id().is_some() {
        send_ha_notification(config, true).await?;
        return Ok(());
    }

    let mut first_err: Option<anyhow::Error> = None;
    if config.ac_entity_id().is_some() {
        if let Err(err) = send_ha_action(config, HaAction::ToggleAc { on: true }).await {
            first_err = Some(err);
        }
    }

    if config.ambient_light_entity_id().is_some()
        || config.main_light_entity_id().is_some()
        || config.door_sign_light_entity_id().is_some()
    {
        for entity_id in configured_light_entity_ids(config).into_iter().flatten() {
            if let Err(err) = send_entity_toggle_request(config, entity_id, true).await {
                first_err.get_or_insert(err);
            }
        }
    }

    match first_err {
        Some(err) => Err(err),
        None => Ok(()),
    }
}
```

- [ ] **Step 4: Run the tests and verify they pass**

Run: `cargo test startup_with_pc_entity_sends_only_pc_online`
Run: `cargo test startup_without_pc_entity_keeps_direct_control`

Expected: PASS.

### Task 2: Gate shutdown behavior on `pc_entity_id`

**Files:**
- Modify: `src-tauri/src/action.rs`
- Test: `src-tauri/src/action.rs`

- [ ] **Step 1: Write the failing tests**

```rust
#[tokio::test]
async fn shutdown_with_pc_entity_sends_only_pc_offline() {
    // expect a POST to /api/services/input_boolean/turn_off
    // with body {"entity_id":"input_boolean.pc_05_online"}
    // and no AC/light turn_off requests
}

#[tokio::test]
async fn shutdown_without_pc_entity_keeps_direct_control() {
    // expect the existing AC/light shutdown requests
    // and no PC offline request
}
```

- [ ] **Step 2: Run the tests and verify they fail**

Run: `cargo test shutdown_with_pc_entity_sends_only_pc_offline`
Run: `cargo test shutdown_without_pc_entity_keeps_direct_control`

Expected: FAIL because `send_shutdown_signal` still routes shutdown as a mixed PC/device action.

- [ ] **Step 3: Implement the minimal code**

```rust
pub(crate) async fn send_shutdown_signal(config: &AppConfig) -> Result<()> {
    if config.pc_entity_id().is_some() {
        send_ha_notification(config, false).await?;
        return Ok(());
    }

    let mut first_err: Option<anyhow::Error> = None;
    if config.ac_entity_id().is_some() {
        if let Err(err) = send_ha_action(config, HaAction::ToggleAc { on: false }).await {
            first_err = Some(err);
        }
    }

    for entity_id in configured_light_entity_ids(config).into_iter().flatten() {
        if let Err(err) = send_entity_toggle_request(config, entity_id, false).await {
            first_err.get_or_insert(err);
        }
    }

    match first_err {
        Some(err) => Err(err),
        None => Ok(()),
    }
}
```

- [ ] **Step 4: Run the tests and verify they pass**

Run: `cargo test shutdown_with_pc_entity_sends_only_pc_offline`
Run: `cargo test shutdown_without_pc_entity_keeps_direct_control`

Expected: PASS.

### Task 3: Update README behavior notes

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update the behavior text**

```md
- If `pc_entity_id` is configured, startup and shutdown only report PC state and Home Assistant handles room-level device control.
- If `pc_entity_id` is not configured, the app keeps the current direct-control fallback for AC and lights.
```

- [ ] **Step 2: Verify the docs match the code path**

Confirm the README no longer implies that startup always turns on devices when `pc_entity_id` exists.

### Task 4: Full verification

**Files:**
- No further code changes expected

- [ ] **Step 1: Run Rust tests**

Run: `cargo test`

Expected: PASS.

- [ ] **Step 2: Run frontend checks**

Run: `npm run lint`

Expected: PASS.

- [ ] **Step 3: Run the frontend build**

Run: `npm run build`

Expected: PASS.
