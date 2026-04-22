# PC Entity Gated Startup/Shutdown Design

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `pc_entity_id` the switch that decides whether the app only reports machine state or also directly controls devices during startup and shutdown.

**Architecture:** When `pc_entity_id` is configured, the app becomes state-only: startup marks the PC online and shutdown marks it offline, but neither path directly toggles AC or lights. When `pc_entity_id` is missing, the existing direct-control fallback stays in place so current single-node setups keep working. This keeps multi-room HA setups isolated while preserving compatibility for configs that have no PC state entity.

**Tech Stack:** Rust, Tauri v1, React, Cargo tests

---

## Behavior

### With `pc_entity_id`

- Startup:
  - send the PC online notification
  - do not turn on AC or lights from the app
- Shutdown:
  - send the PC offline notification
  - do not turn off AC or lights from the app

### Without `pc_entity_id`

- Startup:
  - keep the current direct-control startup behavior for configured AC/lights
- Shutdown:
  - keep the current direct-control shutdown behavior for configured AC/lights

## Files

- Modify: `src-tauri/src/action.rs`
- Test: `src-tauri/src/action.rs`
- Modify: `README.md`

## Tasks

### Task 1: Gate startup behavior on `pc_entity_id`

**Files:**
- Modify: `src-tauri/src/action.rs`
- Test: `src-tauri/src/action.rs`

- [ ] **Step 1: Add failing tests for startup with and without `pc_entity_id`**

```rust
#[tokio::test]
async fn startup_with_pc_entity_sends_only_pc_online() {
    // server should see only:
    // POST /api/services/input_boolean/turn_on
    // body {"entity_id":"input_boolean.pc_05_online"}
    // and no AC/light turn_on requests
}

#[tokio::test]
async fn startup_without_pc_entity_keeps_direct_control() {
    // server should see the existing AC/light startup requests
    // and no PC online request
}
```

- [ ] **Step 2: Run the targeted test to verify it fails**

Run: `cargo test startup_with_pc_entity_sends_only_pc_online`
Run: `cargo test startup_without_pc_entity_keeps_direct_control`

Expected: FAIL because startup currently still couples PC state with device power-on behavior.

- [ ] **Step 3: Update `send_startup_online` to branch on `pc_entity_id`**

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

- [ ] **Step 4: Run the targeted test to verify it passes**

Run: `cargo test startup_with_pc_entity_sends_only_pc_online`
Run: `cargo test startup_without_pc_entity_keeps_direct_control`

Expected: PASS.

### Task 2: Gate shutdown behavior on `pc_entity_id`

**Files:**
- Modify: `src-tauri/src/action.rs`
- Test: `src-tauri/src/action.rs`

- [ ] **Step 1: Add failing tests for shutdown with and without `pc_entity_id`**

```rust
#[tokio::test]
async fn shutdown_with_pc_entity_sends_only_pc_offline() {
    // server should see only:
    // POST /api/services/input_boolean/turn_off
    // body {"entity_id":"input_boolean.pc_05_online"}
    // and no AC/light turn_off requests
}

#[tokio::test]
async fn shutdown_without_pc_entity_keeps_direct_control() {
    // server should see the existing AC/light shutdown requests
    // and no PC offline request
}
```

- [ ] **Step 2: Run the targeted test to verify it fails**

Run: `cargo test shutdown_with_pc_entity_sends_only_pc_offline`
Run: `cargo test shutdown_without_pc_entity_keeps_direct_control`

Expected: FAIL because shutdown currently still mixes the two behaviors.

- [ ] **Step 3: Update `send_shutdown_signal` to branch on `pc_entity_id`**

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

- [ ] **Step 4: Run the targeted test to verify it passes**

Run: `cargo test shutdown_with_pc_entity_sends_only_pc_offline`
Run: `cargo test shutdown_without_pc_entity_keeps_direct_control`

Expected: PASS.

### Task 3: Update docs to reflect the split

**Files:**
- Modify: `README.md`

- [ ] **Step 1: Update the setup guidance**

```md
- If `pc_entity_id` is configured, the app only reports online/offline state and Home Assistant handles room-level device control.
- If `pc_entity_id` is not configured, the app keeps the current direct-control fallback for AC and lights.
```

- [ ] **Step 2: Run a quick docs review**

Confirm the README wording matches the code behavior exactly and does not mention `shutdown_pending`.

## Verification

- `cargo test`
- `npm run lint`
- `npm run build`
