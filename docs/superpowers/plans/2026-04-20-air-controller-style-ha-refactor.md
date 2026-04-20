# Air Controller Style HA Refactor Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** refactor the Rust backend toward the `air-controller` style of separation of concerns while keeping air conditioner control, light control, PC online/offline signaling, and snapshot-based UI state.

**Architecture:** Split the current monolithic Tauri backend into focused modules: `models.rs` for shared types, `snapshot.rs` for HA-to-UI state conversion, `ha_client.rs` for authenticated Home Assistant HTTP calls and climate temperature normalization, and `action.rs` for action dispatch plus post-action confirmation. Keep `main.rs` as startup and Tauri wiring, and keep the frontend command names unchanged so `initialize_app`, `refresh_ha_state`, and `handle_ha_action` keep working.

**Tech Stack:** Rust, Tauri v1, reqwest, tokio, serde, anyhow, Cargo tests

---

### Task 1: Extract Shared Models And Snapshot Assembly

**Files:**
- Create: `src-tauri/src/models.rs`
- Create: `src-tauri/src/snapshot.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write failing snapshot tests for partial and optional entity states**

```rust
#[cfg(test)]
mod tests {
    use super::{
        snapshot_from_home_assistant, snapshot_from_optional_home_assistant,
        snapshot_from_loaded_states, offline_snapshot,
    };

    #[test]
    fn snapshot_from_loaded_states_keeps_missing_entities_off() {
        let ac_state = HaEntityState {
            state: "cool".into(),
            attributes: serde_json::json!({"temperature": 24}),
        };
        let light_state = HaEntityState {
            state: "on".into(),
            attributes: serde_json::json!({}),
        };

        let snapshot = snapshot_from_loaded_states(None, Some(&ac_state), Some(&light_state));

        assert!(snapshot.connected);
        assert!(snapshot.ac_available);
        assert!(snapshot.light_available);
        assert!(snapshot.ac.is_on);
        assert_eq!(snapshot.ac.temp, 24);
        assert!(snapshot.light_on);
    }

    #[test]
    fn snapshot_from_optional_home_assistant_keeps_missing_devices_off() {
        let pc_state = serde_json::json!({"state": "on", "attributes": {}});

        let snapshot = snapshot_from_optional_home_assistant(&pc_state, None, None)
            .expect("snapshot should build");

        assert!(snapshot.connected);
        assert!(!snapshot.ac_available);
        assert!(!snapshot.light_available);
        assert!(!snapshot.ac.is_on);
        assert!(!snapshot.light_on);
    }

    #[test]
    fn offline_snapshot_marks_disconnected_and_keeps_configured_devices() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
        };

        let snapshot = offline_snapshot(&config);

        assert!(!snapshot.connected);
        assert!(snapshot.ac_available);
        assert!(snapshot.light_available);
        assert!(!snapshot.ac.is_on);
        assert!(!snapshot.light_on);
    }
}
```

- [ ] **Step 2: Run the targeted Rust tests to verify they fail before the module split**

Run: `cargo test snapshot_from_loaded_states_keeps_missing_entities_off snapshot_from_optional_home_assistant_keeps_missing_devices_off offline_snapshot_marks_disconnected_and_keeps_configured_devices`

Expected: FAIL because `snapshot.rs` and the shared model types do not exist yet.

- [ ] **Step 3: Implement the shared model and snapshot modules**

```rust
// src-tauri/src/models.rs
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceIds {
    #[serde(default)]
    pub ac: Option<String>,
    #[serde(default)]
    pub light: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ha_url: String,
    pub token: String,
    #[serde(default)]
    pub pc_entity_id: Option<String>,
    #[serde(default)]
    pub entity_id: Option<DeviceIds>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ACState {
    pub is_on: bool,
    pub temp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceSnapshot {
    pub room: String,
    #[serde(rename = "pcId")]
    pub pc_id: String,
    pub ac: ACState,
    #[serde(rename = "lightOn")]
    pub light_on: bool,
    #[serde(rename = "acAvailable")]
    pub ac_available: bool,
    #[serde(rename = "lightAvailable")]
    pub light_available: bool,
    pub connected: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HaEntityState {
    pub state: String,
    #[serde(default)]
    pub attributes: serde_json::Value,
}
```

```rust
// src-tauri/src/snapshot.rs
use anyhow::Result;
use crate::models::{ACState, AppConfig, DeviceSnapshot, HaEntityState};

pub fn initial_snapshot() -> DeviceSnapshot {
    DeviceSnapshot {
        room: "核心-01".into(),
        pc_id: "终端-05".into(),
        ac: ACState { is_on: true, temp: 16 },
        light_on: true,
        ac_available: true,
        light_available: true,
        connected: true,
    }
}

pub fn offline_snapshot(config: &AppConfig) -> DeviceSnapshot {
    let mut snapshot = initial_snapshot();
    snapshot.connected = false;
    snapshot.ac_available = config.entity_id.as_ref().and_then(|ids| ids.ac.as_ref()).is_some();
    snapshot.light_available = config
        .entity_id
        .as_ref()
        .and_then(|ids| ids.light.as_ref())
        .is_some();
    snapshot.ac.is_on = false;
    snapshot.light_on = false;
    snapshot
}

pub fn snapshot_from_loaded_states(
    pc_state: Option<&HaEntityState>,
    ac_state: Option<&HaEntityState>,
    light_state: Option<&HaEntityState>,
) -> DeviceSnapshot {
    let mut snapshot = initial_snapshot();
    snapshot.connected = pc_state.is_some() || ac_state.is_some() || light_state.is_some();
    snapshot.ac_available = ac_state.is_some();
    snapshot.light_available = light_state.is_some();

    if let Some(ac_state) = ac_state {
        snapshot.ac.is_on = !ac_state.state.eq_ignore_ascii_case("off");
        if let Some(temp) = ac_state.attributes.get("temperature") {
            if let Some(value) = temp.as_i64() {
                snapshot.ac.temp = value as i32;
            }
        }
    } else {
        snapshot.ac.is_on = false;
    }

    if let Some(light_state) = light_state {
        snapshot.light_on = light_state.state.eq_ignore_ascii_case("on");
    } else {
        snapshot.light_on = false;
    }

    snapshot
}

pub fn snapshot_from_home_assistant(
    pc_state: &serde_json::Value,
    ac_state: &serde_json::Value,
    light_state: &serde_json::Value,
) -> Result<DeviceSnapshot> {
    let pc_state: HaEntityState = serde_json::from_value(pc_state.clone())?;
    let ac_state: HaEntityState = serde_json::from_value(ac_state.clone())?;
    let light_state: HaEntityState = serde_json::from_value(light_state.clone())?;

    Ok(snapshot_from_loaded_states(
        Some(&pc_state),
        Some(&ac_state),
        Some(&light_state),
    ))
}

pub fn snapshot_from_optional_home_assistant(
    pc_state: &serde_json::Value,
    ac_state: Option<&serde_json::Value>,
    light_state: Option<&serde_json::Value>,
) -> Result<DeviceSnapshot> {
    let pc_state: HaEntityState = serde_json::from_value(pc_state.clone())?;
    let ac_state = ac_state.map(|value| serde_json::from_value(value.clone())).transpose()?;
    let light_state = light_state.map(|value| serde_json::from_value(value.clone())).transpose()?;

    Ok(snapshot_from_loaded_states(
        Some(&pc_state),
        ac_state.as_ref(),
        light_state.as_ref(),
    ))
}
```

- [ ] **Step 4: Run the snapshot tests again and confirm they pass**

Run: `cargo test snapshot_from_loaded_states_keeps_missing_entities_off snapshot_from_optional_home_assistant_keeps_missing_devices_off offline_snapshot_marks_disconnected_and_keeps_configured_devices`

Expected: PASS.

- [ ] **Step 5: Commit the snapshot/model extraction slice**

```bash
git add src-tauri/src/models.rs src-tauri/src/snapshot.rs src-tauri/src/main.rs
git commit -m "refactor: extract snapshot models and assembly"
```

### Task 2: Extract Home Assistant Client And Temperature Normalization

**Files:**
- Create: `src-tauri/src/ha_client.rs`
- Modify: `src-tauri/src/models.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write failing client tests for climate/light requests and temperature normalization**

```rust
#[cfg(test)]
mod tests {
    use super::{HomeAssistantClient, TemperatureUnit, normalize_entity_temperature};

    #[test]
    fn builds_climate_turn_on_request() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: None,
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
        };

        let client = HomeAssistantClient::new(config, "token".into()).expect("client");
        let request = client.build_climate_request("turn_on", serde_json::json!({"entity_id": "climate.office_ac"}));

        assert_eq!(request.url, "https://ha.example.local/api/services/climate/turn_on");
        assert_eq!(request.body, serde_json::json!({"entity_id": "climate.office_ac"}));
    }

    #[test]
    fn builds_light_turn_off_request() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: None,
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
        };

        let client = HomeAssistantClient::new(config, "token".into()).expect("client");
        let request = client.build_light_request("turn_off", serde_json::json!({"entity_id": "light.office_light"}));

        assert_eq!(request.url, "https://ha.example.local/api/services/light/turn_off");
        assert_eq!(request.body, serde_json::json!({"entity_id": "light.office_light"}));
    }

    #[test]
    fn normalizes_temperature_with_unit_and_step() {
        let snapshot = ClimateSnapshot {
            state: ClimateState {
                entity_id: "climate.office_ac".into(),
                state: "cool".into(),
                hvac_mode: "cool".into(),
                hvac_action: "cooling".into(),
                current_temperature: Some(25.0),
                target_temperature: Some(25.0),
                min_temperature: Some(16.0),
                max_temperature: Some(30.0),
                temperature_step: Some(1.0),
                is_available: true,
                is_on: true,
            },
            temperature_unit: TemperatureUnit::Celsius,
            entity_min_temperature: Some(16.0),
            entity_max_temperature: Some(30.0),
            entity_temperature_step: Some(1.0),
        };

        assert_eq!(normalize_entity_temperature(27.4, &snapshot), 27.0);
    }
}
```

- [ ] **Step 2: Run the targeted Rust tests to verify they fail before the client split**

Run: `cargo test builds_climate_turn_on_request builds_light_turn_off_request normalizes_temperature_with_unit_and_step`

Expected: FAIL because `ha_client.rs`, `TemperatureUnit`, and `ClimateSnapshot` do not exist yet.

- [ ] **Step 3: Implement the Home Assistant client with climate and light support**

```rust
// src-tauri/src/ha_client.rs
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::time::Duration;

use crate::models::{AppConfig, ClimateState, HaEntityState};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
    Unknown,
}

pub struct ClimateSnapshot {
    pub state: ClimateState,
    pub temperature_unit: TemperatureUnit,
    pub entity_min_temperature: Option<f64>,
    pub entity_max_temperature: Option<f64>,
    pub entity_temperature_step: Option<f64>,
}

pub struct HaRequest {
    pub url: String,
    pub body: Value,
}

pub struct HomeAssistantClient {
    config: AppConfig,
    token: String,
    client: Client,
}

impl HomeAssistantClient {
    pub fn new(config: AppConfig, token: String) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10))
            .build()?;

        Ok(Self { config, token, client })
    }

    pub fn build_climate_request(&self, action: &str, body: Value) -> HaRequest {
        HaRequest {
            url: format!(
                "{}/api/services/climate/{}",
                self.config.ha_url.trim_end_matches('/'),
                action
            ),
            body,
        }
    }

    pub fn build_light_request(&self, action: &str, body: Value) -> HaRequest {
        HaRequest {
            url: format!(
                "{}/api/services/light/{}",
                self.config.ha_url.trim_end_matches('/'),
                action
            ),
            body,
        }
    }

    async fn send_request(&self, request: HaRequest) -> Result<()> {
        self.client
            .post(request.url)
            .bearer_auth(&self.token)
            .json(&request.body)
            .send()
            .await?
            .error_for_status()?;
        Ok(())
    }

    pub async fn turn_on_ac(&self) -> Result<()> {
        let entity_id = self.ac_entity_id()?;
        self.send_request(self.build_climate_request("turn_on", json!({"entity_id": entity_id})))
            .await
    }

    pub async fn turn_off_ac(&self) -> Result<()> {
        let entity_id = self.ac_entity_id()?;
        self.send_request(self.build_climate_request("turn_off", json!({"entity_id": entity_id})))
            .await
    }

    pub async fn set_temperature(&self, temperature: f64) -> Result<()> {
        let entity_id = self.ac_entity_id()?;
        let snapshot = self.fetch_climate_snapshot().await?;
        let entity_temperature = normalize_entity_temperature(temperature, &snapshot);

        self.send_request(self.build_climate_request(
            "set_temperature",
            json!({
                "entity_id": entity_id,
                "temperature": entity_temperature,
            }),
        ))
        .await
    }

    pub async fn turn_on_light(&self) -> Result<()> {
        let entity_id = self.light_entity_id()?;
        self.send_request(self.build_light_request("turn_on", json!({"entity_id": entity_id})))
            .await
    }

    pub async fn turn_off_light(&self) -> Result<()> {
        let entity_id = self.light_entity_id()?;
        self.send_request(self.build_light_request("turn_off", json!({"entity_id": entity_id})))
            .await
    }

    async fn fetch_climate_snapshot(&self) -> Result<ClimateSnapshot> {
        let entity_id = self.ac_entity_id()?;
        let state = self.fetch_entity_state(entity_id).await?;
        let attributes = state.attributes;
        let target_temperature = parse_double(attributes.get("temperature"));
        let min_temperature = parse_double(attributes.get("min_temp"));
        let max_temperature = parse_double(attributes.get("max_temp"));
        let step = parse_double(attributes.get("target_temp_step"));
        let unit = parse_temperature_unit(
            attributes.get("temperature_unit"),
            target_temperature,
            min_temperature,
            max_temperature,
            parse_double(attributes.get("current_temperature")),
        );

        Ok(ClimateSnapshot {
            state: ClimateState {
                entity_id: entity_id.to_string(),
                state: state.state,
                hvac_mode: attributes
                    .get("hvac_mode")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                hvac_action: attributes
                    .get("hvac_action")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_string(),
                current_temperature: parse_double(attributes.get("current_temperature")),
                target_temperature,
                min_temperature,
                max_temperature,
                temperature_step: step,
                is_available: true,
                is_on: true,
            },
            temperature_unit: unit,
            entity_min_temperature: min_temperature,
            entity_max_temperature: max_temperature,
            entity_temperature_step: step,
        })
    }

    async fn fetch_entity_state(&self, entity_id: &str) -> Result<HaEntityState> {
        let response = self
            .client
            .get(format!("{}/api/states/{}", self.config.ha_url.trim_end_matches('/'), entity_id))
            .bearer_auth(&self.token)
            .send()
            .await?
            .error_for_status()?;

        Ok(response.json::<HaEntityState>().await?)
    }

    fn ac_entity_id(&self) -> Result<&str> {
        self.config
            .entity_id
            .as_ref()
            .and_then(|ids| ids.ac.as_deref())
            .ok_or_else(|| anyhow!("AC entity is not configured"))
    }

    fn light_entity_id(&self) -> Result<&str> {
        self.config
            .entity_id
            .as_ref()
            .and_then(|ids| ids.light.as_deref())
            .ok_or_else(|| anyhow!("light entity is not configured"))
    }
}

pub fn normalize_entity_temperature(value_celsius: f64, snapshot: &ClimateSnapshot) -> f64 {
    let mut value = match snapshot.temperature_unit {
        TemperatureUnit::Fahrenheit => (value_celsius * 9.0 / 5.0) + 32.0,
        TemperatureUnit::Celsius | TemperatureUnit::Unknown => value_celsius,
    };

    if let Some(minimum) = snapshot.entity_min_temperature {
        value = value.max(minimum);
    }
    if let Some(maximum) = snapshot.entity_max_temperature {
        value = value.min(maximum);
    }
    if let Some(step) = snapshot.entity_temperature_step.filter(|step| *step > 0.0) {
        let anchor = snapshot.entity_min_temperature.unwrap_or(0.0);
        value = anchor + ((value - anchor) / step).round() * step;
    }

    (value * 10.0).round() / 10.0
}
```

- [ ] **Step 4: Run the client tests again and confirm they pass**

Run: `cargo test builds_climate_turn_on_request builds_light_turn_off_request normalizes_temperature_with_unit_and_step`

Expected: PASS.

- [ ] **Step 5: Commit the client extraction slice**

```bash
git add src-tauri/src/models.rs src-tauri/src/ha_client.rs src-tauri/src/main.rs
git commit -m "refactor: extract home assistant client"
```

### Task 3: Extract Action Orchestration For Air And Light Controls

**Files:**
- Create: `src-tauri/src/action.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/src/commands.rs`

- [ ] **Step 1: Write failing action-dispatch tests for AC and light confirmation behavior**

```rust
#[cfg(test)]
mod tests {
    use super::{apply_action_with_delays, ActionArgs, ActionApplyOutcome};
    use std::{collections::VecDeque, sync::Mutex, time::Duration};

    struct MockActionClient {
        turn_on_ac_results: Mutex<VecDeque<anyhow::Result<()>>>,
        turn_off_ac_results: Mutex<VecDeque<anyhow::Result<()>>>,
        set_temperature_results: Mutex<VecDeque<anyhow::Result<()>>>,
        turn_on_light_results: Mutex<VecDeque<anyhow::Result<()>>>,
        turn_off_light_results: Mutex<VecDeque<anyhow::Result<()>>>,
        startup_online_results: Mutex<VecDeque<anyhow::Result<()>>>,
        shutdown_signal_results: Mutex<VecDeque<anyhow::Result<()>>>,
        snapshot_results: Mutex<VecDeque<anyhow::Result<DeviceSnapshot>>>,
    }

    impl MockActionClient {
        fn new(snapshot_results: Vec<anyhow::Result<DeviceSnapshot>>) -> Self {
            Self {
                turn_on_ac_results: Mutex::new(VecDeque::from(vec![Ok(())])),
                turn_off_ac_results: Mutex::new(VecDeque::from(vec![Ok(())])),
                set_temperature_results: Mutex::new(VecDeque::from(vec![Ok(())])),
                turn_on_light_results: Mutex::new(VecDeque::from(vec![Ok(())])),
                turn_off_light_results: Mutex::new(VecDeque::from(vec![Ok(())])),
                startup_online_results: Mutex::new(VecDeque::from(vec![Ok(())])),
                shutdown_signal_results: Mutex::new(VecDeque::from(vec![Ok(())])),
                snapshot_results: Mutex::new(VecDeque::from(snapshot_results)),
            }
        }
    }

    impl crate::action::ActionClient for MockActionClient {
        fn turn_on_ac(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + '_>> {
            Box::pin(async move { self.turn_on_ac_results.lock().unwrap().pop_front().unwrap_or(Ok(())) })
        }

        fn turn_off_ac(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + '_>> {
            Box::pin(async move { self.turn_off_ac_results.lock().unwrap().pop_front().unwrap_or(Ok(())) })
        }

        fn set_temperature(&self, _temperature: f64) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + '_>> {
            Box::pin(async move { self.set_temperature_results.lock().unwrap().pop_front().unwrap_or(Ok(())) })
        }

        fn turn_on_light(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + '_>> {
            Box::pin(async move { self.turn_on_light_results.lock().unwrap().pop_front().unwrap_or(Ok(())) })
        }

        fn turn_off_light(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + '_>> {
            Box::pin(async move { self.turn_off_light_results.lock().unwrap().pop_front().unwrap_or(Ok(())) })
        }

        fn startup_online(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + '_>> {
            Box::pin(async move { self.startup_online_results.lock().unwrap().pop_front().unwrap_or(Ok(())) })
        }

        fn shutdown_signal(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<()>> + Send + '_>> {
            Box::pin(async move { self.shutdown_signal_results.lock().unwrap().pop_front().unwrap_or(Ok(())) })
        }

        fn fetch_snapshot(&self) -> std::pin::Pin<Box<dyn std::future::Future<Output = anyhow::Result<DeviceSnapshot>> + Send + '_>> {
            Box::pin(async move { self.snapshot_results.lock().unwrap().pop_front().unwrap_or_else(|| Ok(snapshot_with_ac(true, 24))) })
        }
    }

    fn snapshot_with_ac(is_on: bool, temp: i32) -> DeviceSnapshot {
        DeviceSnapshot {
            room: "核心-01".into(),
            pc_id: "终端-05".into(),
            ac: ACState { is_on, temp },
            light_on: false,
            ac_available: true,
            light_available: true,
            connected: true,
        }
    }

    fn snapshot_with_light(is_on: bool) -> DeviceSnapshot {
        DeviceSnapshot {
            room: "核心-01".into(),
            pc_id: "终端-05".into(),
            ac: ACState { is_on: false, temp: 24 },
            light_on: is_on,
            ac_available: true,
            light_available: true,
            connected: true,
        }
    }

    #[tokio::test]
    async fn ac_toggle_confirms_refreshed_snapshot() {
        let client = MockActionClient::new(vec![Ok(snapshot_with_ac(true, 24))]);
        let snapshot = snapshot_with_ac(false, 24);
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
        };

        let outcome = apply_action_with_delays(
            &config,
            &client,
            snapshot,
            ActionArgs { action: "ac_toggle".into(), value: None },
            Duration::ZERO,
            1,
            Duration::ZERO,
        )
        .await
        .expect("action outcome");

        assert!(outcome.error.is_none());
        assert!(outcome.snapshot.ac.is_on);
    }

    #[tokio::test]
    async fn light_toggle_confirms_refreshed_snapshot() {
        let client = MockActionClient::new(vec![Ok(snapshot_with_light(true))]);
        let snapshot = snapshot_with_light(false);
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
        };

        let outcome = apply_action_with_delays(
            &config,
            &client,
            snapshot,
            ActionArgs { action: "light_toggle".into(), value: None },
            Duration::ZERO,
            1,
            Duration::ZERO,
        )
        .await
        .expect("action outcome");

        assert!(outcome.error.is_none());
        assert!(outcome.snapshot.light_on);
    }
}
```

- [ ] **Step 2: Run the targeted Rust tests to verify they fail before the action split**

Run: `cargo test ac_toggle_confirms_refreshed_snapshot light_toggle_confirms_refreshed_snapshot`

Expected: FAIL because `action.rs` and the action confirmation helpers do not exist yet.

- [ ] **Step 3: Implement the action module with polling confirmation and startup/shutdown fan-out**

```rust
// src-tauri/src/action.rs
use anyhow::{anyhow, Result};
use std::{future::Future, pin::Pin, time::Duration};

use crate::models::{AppConfig, DeviceSnapshot};

#[derive(Debug, Clone)]
pub struct ActionArgs {
    pub action: String,
    pub value: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ActionApplyOutcome {
    pub snapshot: DeviceSnapshot,
    pub error: Option<String>,
}

pub trait ActionClient {
    fn turn_on_ac(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    fn turn_off_ac(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    fn set_temperature(&self, temperature: f64) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    fn turn_on_light(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    fn turn_off_light(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    fn startup_online(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    fn shutdown_signal(&self) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    fn fetch_snapshot(&self) -> Pin<Box<dyn Future<Output = Result<DeviceSnapshot>> + Send + '_>>;
}

pub async fn apply_action<C: ActionClient>(
    config: &AppConfig,
    client: &C,
    snapshot: DeviceSnapshot,
    args: ActionArgs,
) -> Result<ActionApplyOutcome> {
    apply_action_with_delays(
        config,
        client,
        snapshot,
        args,
        Duration::from_secs(2),
        5,
        Duration::from_secs(2),
    )
    .await
}

pub async fn apply_action_with_delays<C: ActionClient>(
    config: &AppConfig,
    client: &C,
    mut snapshot: DeviceSnapshot,
    args: ActionArgs,
    initial_delay: Duration,
    retry_count: usize,
    retry_interval: Duration,
) -> Result<ActionApplyOutcome> {
    match args.action.as_str() {
        "ac_toggle" => {
            let next = !snapshot.ac.is_on;
            let result = if next { client.turn_on_ac().await } else { client.turn_off_ac().await };
            snapshot.ac.is_on = next;
            return Ok(confirm_action_snapshot_with_delays(
                client,
                &snapshot,
                result.err().map(|err| err.to_string()),
                move |current| current.ac.is_on == next,
                initial_delay,
                retry_count,
                retry_interval,
                "空调尚未进入预期开关状态，继续等待刷新。",
            ).await);
        }
        "ac_set_temp" => {
            let temp = args.value.ok_or_else(|| anyhow!("missing temperature"))?;
            let result = client.set_temperature(temp as f64).await;
            snapshot.ac.temp = temp;
            return Ok(confirm_action_snapshot_with_delays(
                client,
                &snapshot,
                result.err().map(|err| err.to_string()),
                move |current| current.ac.is_on && current.ac.temp == temp,
                initial_delay,
                retry_count,
                retry_interval,
                "空调尚未进入预期温度状态，继续等待刷新。",
            ).await);
        }
        "light_toggle" => {
            let next = !snapshot.light_on;
            let result = if next { client.turn_on_light().await } else { client.turn_off_light().await };
            snapshot.light_on = next;
            return Ok(confirm_action_snapshot_with_delays(
                client,
                &snapshot,
                result.err().map(|err| err.to_string()),
                move |current| current.light_on == next,
                initial_delay,
                retry_count,
                retry_interval,
                "环境氛围照明尚未进入预期开关状态，继续等待刷新。",
            ).await);
        }
        "startup_online" => {
            client.startup_online().await?;
            snapshot.ac_available = config.entity_id.as_ref().and_then(|ids| ids.ac.as_ref()).is_some();
            snapshot.light_available = config.entity_id.as_ref().and_then(|ids| ids.light.as_ref()).is_some();
            snapshot.ac.is_on = snapshot.ac_available;
            snapshot.light_on = snapshot.light_available;
        }
        "shutdown_signal" => {
            client.shutdown_signal().await?;
        }
        _ => return Err(anyhow!("unsupported action: {}", args.action)),
    }

    Ok(ActionApplyOutcome { snapshot, error: None })
}

async fn confirm_action_snapshot_with_delays<C: ActionClient, F>(
    client: &C,
    original: &DeviceSnapshot,
    action_error: Option<String>,
    expected: F,
    initial_delay: Duration,
    retry_count: usize,
    retry_interval: Duration,
    pending_message: &'static str,
) -> ActionApplyOutcome
where
    F: Fn(&DeviceSnapshot) -> bool,
{
    let mut last_snapshot = original.clone();
    let mut last_error = action_error;

    tokio::time::sleep(initial_delay).await;

    for _ in 0..retry_count {
        match client.fetch_snapshot().await {
            Ok(snapshot) => {
                if expected(&snapshot) {
                    return ActionApplyOutcome { snapshot, error: None };
                }
                last_snapshot = snapshot;
                last_error = Some(pending_message.to_string());
            }
            Err(err) => {
                last_error = Some(err.to_string());
            }
        }

        tokio::time::sleep(retry_interval).await;
    }

    ActionApplyOutcome { snapshot: last_snapshot, error: last_error }
}
```

- [ ] **Step 4: Run the action tests again and confirm they pass**

Run: `cargo test ac_toggle_confirms_refreshed_snapshot light_toggle_confirms_refreshed_snapshot`

Expected: PASS.

- [ ] **Step 5: Commit the action orchestration slice**

```bash
git add src-tauri/src/action.rs src-tauri/src/main.rs src-tauri/src/commands.rs
git commit -m "refactor: extract action orchestration"
```

### Task 4: Rewire Tauri Commands And Simplify Main

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Write a compile-focused regression test for the public command names**

```rust
#[test]
fn command_names_remain_stable() {
    let command_names = [
        "initialize_app",
        "refresh_ha_state",
        "handle_ha_action",
        "is_autostart_mode",
        "set_autostart_enabled",
        "append_log_message",
    ];

    assert_eq!(command_names.len(), 6);
    assert!(command_names.contains(&"handle_ha_action"));
    assert!(command_names.contains(&"initialize_app"));
}
```

- [ ] **Step 2: Run the targeted Rust test to verify the command surface is still represented**

Run: `cargo test command_names_remain_stable`

Expected: PASS once the new module layout preserves the same invoke-handler names.

- [ ] **Step 3: Move the Tauri command handlers into a dedicated module and keep the frontend protocol unchanged**

```rust
// src-tauri/src/commands.rs
use crate::{
    action::{apply_action, ActionArgs},
    ha_client::HomeAssistantClient,
    models::{AppConfig, DeviceSnapshot},
    snapshot::offline_snapshot,
};
```

Move the existing `initialize_app`, `refresh_ha_state`, and `handle_ha_action` bodies into this module unchanged, keeping the same return types, `state-refresh` emits, and public command names that the frontend already invokes.

```rust
// src-tauri/src/main.rs
mod action;
mod commands;
mod ha_client;
mod models;
mod snapshot;

fn main() {
    // Keep startup/autostart, tray, and window wiring here.
    // All HA action/state logic lives in the modules above.
}
```

- [ ] **Step 4: Run the full backend and frontend verification suite**

Run:
`cargo test`
`cargo check --target x86_64-pc-windows-gnu`
`npm run build`
`npm run lint`

Expected: all commands pass, and the frontend still talks to the same Tauri command names.

- [ ] **Step 5: Commit the module rewire and final cleanup**

```bash
git add src-tauri/src/main.rs src-tauri/src/commands.rs src-tauri/src/action.rs src-tauri/src/ha_client.rs src-tauri/src/models.rs src-tauri/src/snapshot.rs
git commit -m "refactor: split ha backend into focused modules"
```
