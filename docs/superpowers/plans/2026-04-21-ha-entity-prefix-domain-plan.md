# HA Entity Prefix Domain Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Keep the configuration to a single `entity_id` value per controllable item, and infer the Home Assistant domain from the `entity_id` prefix when building service requests.

**Architecture:** Store only entity IDs in config and derive the domain from the string before the first `.` at request time. Reuse the existing request-building flow so `switch`, `climate`, and similar entities can share one backend path without changing the UI.

**Tech Stack:** Rust, Tauri backend commands, Home Assistant REST API, unit tests with `cargo test`.

---

### Task 1: Add failing prefix-domain tests

**Files:**
- Modify: `src-tauri/src/ha_client.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn generic_request_uses_entity_prefix_as_domain() {
    let config = AppConfig {
        ha_url: "https://ha.example.local".into(),
        token: "secret".into(),
        pc_entity_id: Some("input_boolean.pc_05_online".into()),
        entity_id: Some(DeviceIds {
            ac: Some("climate.office_ac".into()),
            switch: Some("switch.office_light".into()),
        }),
    };

    let request = generic_request(&config, "switch.office_light", "turn_on").expect("request");

    assert_eq!(request.url, "https://ha.example.local/api/services/switch/turn_on");
    assert_eq!(request.body, serde_json::json!({"entity_id": "switch.office_light"}));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test generic_request_uses_entity_prefix_as_domain`
Expected: compile failure because the new helper API does not exist yet.

### Task 2: Implement entity-prefix domain inference

**Files:**
- Modify: `src-tauri/src/ha_client.rs`

- [ ] **Step 1: Write minimal implementation**

```rust
fn entity_domain(entity_id: &str) -> Result<&str> {
    entity_id
        .split_once('.')
        .map(|(domain, _)| domain)
        .ok_or_else(|| anyhow!("entity_id must contain a domain prefix"))
}

fn generic_request(config: &AppConfig, entity_id: &str, service: &str) -> Result<HaRequest> {
    let domain = entity_domain(entity_id)?;

    Ok(HaRequest {
        url: format!("{}/api/services/{}/{}", base_url(config), domain, service),
        body: request_body(entity_id),
    })
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test generic_request_uses_entity_prefix_as_domain`
Expected: PASS.

### Task 3: Route existing helpers through prefix-based generic requests

**Files:**
- Modify: `src-tauri/src/ha_client.rs`

- [ ] **Step 1: Replace the switch/climate request builders**

```rust
fn climate_request(config: &AppConfig, service: &str) -> Result<HaRequest> {
    let entity_id = entity_id(config, true)?;
    generic_request(config, entity_id, service)
}

fn switch_request(config: &AppConfig, service: &str) -> Result<HaRequest> {
    let entity_id = entity_id(config, false)?;
    generic_request(config, entity_id, service)
}
```

- [ ] **Step 2: Keep climate-specific helpers unchanged**

Keep these functions as-is:

```rust
pub async fn normalized_climate_temperature(config: &AppConfig, requested: i32) -> Result<i32> { ... }
pub fn climate_set_temperature_request(config: &AppConfig, temperature: i32) -> Result<HaRequest> { ... }
pub fn normalize_climate_temperature(state: &HaEntityState, requested: i32) -> f64 { ... }
pub async fn climate_temperature_targets(config: &AppConfig, requested: i32) -> Result<(i32, i32)> { ... }
```

- [ ] **Step 3: Run focused request tests**

Run: `cargo test switch_turn climate_turn`
Expected: request construction tests pass.

### Task 4: Regression check

**Files:**
- None

- [ ] **Step 1: Run the full crate test suite**

Run: `cargo test`
Expected: all tests pass with no regressions.
