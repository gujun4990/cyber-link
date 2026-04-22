# HA Domain Internal Abstraction Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a domain-aware Home Assistant request helper inside the Rust backend, while keeping the current config schema and climate-specific logic unchanged.

**Architecture:** Introduce a small `HaDomain` enum plus a generic request builder that composes `/{domain}/{service}` URLs and resolves the configured entity ID per domain. Keep `climate` temperature normalization and action confirmation as dedicated code paths so only repetitive request construction is shared.

**Tech Stack:** Rust, Tauri backend commands, Home Assistant REST API, unit tests with `cargo test`.

---

### Task 1: Add domain-aware request tests

**Files:**
- Modify: `src-tauri/src/ha_client.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[test]
fn generic_request_builds_url_from_domain() {
    let config = AppConfig {
        ha_url: "https://ha.example.local".into(),
        token: "secret".into(),
        pc_entity_id: Some("input_boolean.pc_05_online".into()),
        entity_id: Some(DeviceIds {
            ac: Some("climate.office_ac".into()),
            ambient_light: Some("switch.office_light".into()),
        }),
    };

    let request = generic_request(&config, HaDomain::Switch, "turn_on").expect("request");

    assert_eq!(request.url, "https://ha.example.local/api/services/switch/turn_on");
    assert_eq!(request.body, serde_json::json!({"entity_id": "switch.office_light"}));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test generic_request_builds_url_from_domain`
Expected: compile failure because `HaDomain` and `generic_request` do not exist yet.

### Task 2: Add the generic domain helper

**Files:**
- Modify: `src-tauri/src/ha_client.rs`

- [ ] **Step 1: Write minimal implementation**

```rust
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HaDomain {
    Climate,
    Switch,
    MediaPlayer,
}

impl AsRef<str> for HaDomain {
    fn as_ref(&self) -> &str {
        match self {
            Self::Climate => "climate",
            Self::Switch => "switch",
            Self::MediaPlayer => "media_player",
        }
    }
}

fn get_entity_id_by_domain<'a>(config: &'a AppConfig, domain: &HaDomain) -> Result<&'a str> {
    match domain {
        HaDomain::Climate => entity_id(config, true),
        HaDomain::Switch => entity_id(config, false),
        HaDomain::MediaPlayer => Err(anyhow!("media_player entity is not configured")),
    }
}

fn generic_request(config: &AppConfig, domain: HaDomain, service: &str) -> Result<HaRequest> {
    let entity_id = get_entity_id_by_domain(config, &domain)?;

    Ok(HaRequest {
        url: format!("{}/api/services/{}/{}", base_url(config), domain.as_ref(), service),
        body: request_body(entity_id),
    })
}
```

- [ ] **Step 2: Run test to verify it passes**

Run: `cargo test generic_request_builds_url_from_domain`
Expected: PASS.

### Task 3: Route existing request builders through the generic helper

**Files:**
- Modify: `src-tauri/src/ha_client.rs`

- [ ] **Step 1: Replace the switch/climate request builders**

```rust
fn climate_request(config: &AppConfig, service: &str) -> Result<HaRequest> {
    generic_request(config, HaDomain::Climate, service)
}

fn switch_request(config: &AppConfig, service: &str) -> Result<HaRequest> {
    generic_request(config, HaDomain::Switch, service)
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
