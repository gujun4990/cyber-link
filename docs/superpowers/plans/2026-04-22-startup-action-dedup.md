# Startup Action Dedup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove duplicated `pc_entity_id` branching between startup request sending and startup action snapshot handling.

**Architecture:** Extract the `pc_entity_id`-aware startup behavior into one small Rust helper used by both `send_startup_online` and `ActionKind::StartupOnline`. The helper should keep the current behavior: if `pc_entity_id` exists, only the PC online notification is sent and the snapshot only marks `connected = true`; if it does not exist, the existing device startup fallback remains unchanged. Keep the change local to `src-tauri/src/action.rs` so the behavior stays easy to test.

**Tech Stack:** Rust, Cargo tests

---

### Task 1: Add helper coverage for the current startup branches

**Files:**
- Modify: `src-tauri/src/action.rs`
- Test: `src-tauri/src/action.rs`

- [ ] **Step 1: Write failing tests for both startup modes**

```rust
#[tokio::test]
async fn startup_with_pc_entity_only_marks_connected() {
    // use ActionKind::StartupOnline with pc_entity_id configured
    // expect connected=true and all device states unchanged
}

#[tokio::test]
async fn startup_without_pc_entity_keeps_device_sync() {
    // use ActionKind::StartupOnline with no pc_entity_id
    // expect existing device startup behavior to remain
}
```

- [ ] **Step 2: Run the tests to verify they fail for the right reason**

Run: `cargo test startup_with_pc_entity_only_marks_connected`
Run: `cargo test startup_without_pc_entity_keeps_device_sync`

Expected: FAIL because the two startup paths are still implemented separately and can drift.

- [ ] **Step 3: Add the shared startup helper and route both call sites through it**

```rust
fn apply_startup_snapshot(snapshot: &mut DeviceSnapshot, config: &AppConfig) {
    if config.pc_entity_id().is_some() {
        snapshot.connected = true;
        return;
    }

    snapshot.light_count = config.light_count();
    snapshot.connected = config.ac_entity_id().is_some()
        || config.ambient_light_entity_id().is_some()
        || config.main_light_entity_id().is_some()
        || config.door_sign_light_entity_id().is_some();

    let ac_available = config.ac_entity_id().is_some();
    let ambient_light_available = config.ambient_light_entity_id().is_some();
    let main_light_available = config.main_light_entity_id().is_some();
    let door_sign_light_available = config.door_sign_light_entity_id().is_some();

    snapshot.sync_ac_state(ac_available, ac_available);
    snapshot.sync_ambient_light_state(ambient_light_available, ambient_light_available);
    snapshot.sync_main_light_state(main_light_available, main_light_available);
    snapshot.sync_door_sign_light_state(door_sign_light_available, door_sign_light_available);
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test startup_with_pc_entity_only_marks_connected`
Run: `cargo test startup_without_pc_entity_keeps_device_sync`

Expected: PASS.

### Task 2: Keep `send_startup_online` as request sender only

**Files:**
- Modify: `src-tauri/src/action.rs`
- Test: `src-tauri/src/action.rs`

- [ ] **Step 1: Remove duplicated snapshot branching from `apply_action` and keep request sending isolated**

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

- [ ] **Step 2: Keep the verification commands aligned with the new helper**

Run: `cargo test --quiet`

Expected: PASS.

### Task 3: Full verification

**Files:**
- No further code changes expected

- [ ] **Step 1: Run Rust tests**

Run: `cargo test`

Expected: PASS.
