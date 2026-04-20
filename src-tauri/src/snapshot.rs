use crate::models::{AppConfig, DeviceSnapshot, HaEntityState, LightState};
use crate::temperature::temperature_from_attributes;

pub fn initial_snapshot() -> DeviceSnapshot {
    DeviceSnapshot {
        room: "核心-01".into(),
        pc_id: "终端-05".into(),
        ac: crate::models::ACState {
            is_on: true,
            is_available: true,
            temp: 16,
        },
        light: LightState {
            is_on: true,
            is_available: true,
        },
        light_on: true,
        ac_available: true,
        light_available: true,
        connected: true,
    }
}

pub fn offline_snapshot(_config: &AppConfig) -> DeviceSnapshot {
    let mut snapshot = initial_snapshot();
    snapshot.connected = false;
    snapshot.set_ac_available(false);
    snapshot.set_light_available(false);
    snapshot.set_ac_on(false);
    snapshot.set_light_on(false);

    snapshot
}

pub fn snapshot_from_loaded_states(
    pc_state: Option<&HaEntityState>,
    ac_state: Option<&HaEntityState>,
    light_state: Option<&HaEntityState>,
) -> DeviceSnapshot {
    let mut snapshot = initial_snapshot();
    snapshot.connected = pc_state.is_some() || ac_state.is_some() || light_state.is_some();
    snapshot.set_ac_available(false);
    snapshot.set_light_available(false);

    if let Some(ac_state) = ac_state {
        let is_available = !ac_state.state.eq_ignore_ascii_case("unavailable");
        snapshot.set_ac_available(is_available);
        snapshot.set_ac_on(is_available && !ac_state.state.eq_ignore_ascii_case("off"));

        if is_available {
            if let Some(temp) = temperature_from_attributes(&ac_state.attributes) {
                snapshot.ac.temp = temp;
            }
        }
    } else {
        snapshot.set_ac_available(false);
        snapshot.set_ac_on(false);
    }

    if let Some(light_state) = light_state {
        let is_available = !light_state.state.eq_ignore_ascii_case("unavailable");
        snapshot.set_light_available(is_available);
        snapshot.set_light_on(is_available && light_state.state.eq_ignore_ascii_case("on"));
    } else {
        snapshot.set_light_available(false);
        snapshot.set_light_on(false);
    }

    snapshot
}

fn deserialize_entity_state(value: &serde_json::Value) -> anyhow::Result<HaEntityState> {
    Ok(serde_json::from_value(value.clone())?)
}

fn snapshot_from_json_states(
    pc_state: &serde_json::Value,
    ac_state: Option<&serde_json::Value>,
    light_state: Option<&serde_json::Value>,
) -> anyhow::Result<DeviceSnapshot> {
    let pc_state = deserialize_entity_state(pc_state)?;
    let ac_state = ac_state.map(deserialize_entity_state).transpose()?;
    let light_state = light_state.map(deserialize_entity_state).transpose()?;

    Ok(snapshot_from_loaded_states(
        Some(&pc_state),
        ac_state.as_ref(),
        light_state.as_ref(),
    ))
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn snapshot_from_home_assistant(
    pc_state: &serde_json::Value,
    ac_state: &serde_json::Value,
    light_state: &serde_json::Value,
) -> anyhow::Result<DeviceSnapshot> {
    snapshot_from_json_states(pc_state, Some(ac_state), Some(light_state))
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn snapshot_from_optional_home_assistant(
    pc_state: &serde_json::Value,
    ac_state: Option<&serde_json::Value>,
    light_state: Option<&serde_json::Value>,
) -> anyhow::Result<DeviceSnapshot> {
    snapshot_from_json_states(pc_state, ac_state, light_state)
}

#[cfg(test)]
mod tests {
    use super::{
        offline_snapshot, snapshot_from_home_assistant, snapshot_from_loaded_states,
        snapshot_from_optional_home_assistant,
    };
    use crate::models::{AppConfig, DeviceIds, HaEntityState};

    fn sample_config(ac: Option<&str>, light: Option<&str>) -> AppConfig {
        AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(DeviceIds {
                ac: ac.map(str::to_string),
                light: light.map(str::to_string),
            }),
        }
    }

    #[test]
    fn offline_snapshot_keeps_configured_devices_off() {
        let snapshot = offline_snapshot(&sample_config(
            Some("climate.office_ac"),
            Some("light.office_light"),
        ));

        assert!(!snapshot.connected);
        assert!(!snapshot.ac.is_available);
        assert!(!snapshot.light.is_available);
        assert!(!snapshot.ac_available);
        assert!(!snapshot.light_available);
        assert!(!snapshot.ac.is_on);
        assert!(!snapshot.light.is_on);
        assert!(!snapshot.light_on);
    }

    #[test]
    fn snapshot_from_loaded_states_uses_integer_temperature_and_missing_optional_states_off() {
        let pc_state = HaEntityState {
            state: "on".into(),
            attributes: serde_json::json!({}),
        };
        let ac_state = HaEntityState {
            state: "cool".into(),
            attributes: serde_json::json!({"temperature": 24.6}),
        };

        let snapshot = snapshot_from_loaded_states(Some(&pc_state), Some(&ac_state), None);

        assert!(snapshot.connected);
        assert!(snapshot.ac.is_available);
        assert!(snapshot.ac_available);
        assert!(!snapshot.light.is_available);
        assert!(!snapshot.light_available);
        assert!(snapshot.ac.is_on);
        assert_eq!(snapshot.ac.temp, 25);
        assert!(!snapshot.light_on);
    }

    #[test]
    fn snapshot_from_loaded_states_converts_fahrenheit_temperature_to_celsius() {
        let pc_state = HaEntityState {
            state: "on".into(),
            attributes: serde_json::json!({}),
        };
        let ac_state = HaEntityState {
            state: "cool".into(),
            attributes: serde_json::json!({
                "temperature": 77,
                "temperature_unit": "°F"
            }),
        };

        let snapshot = snapshot_from_loaded_states(Some(&pc_state), Some(&ac_state), None);

        assert!(snapshot.ac.is_available);
        assert_eq!(snapshot.ac.temp, 25);
    }

    #[test]
    fn snapshot_from_loaded_states_marks_unavailable_entities_off() {
        let pc_state = HaEntityState {
            state: "on".into(),
            attributes: serde_json::json!({}),
        };
        let ac_state = HaEntityState {
            state: "unavailable".into(),
            attributes: serde_json::json!({
                "temperature": 24,
                "temperature_unit": "°C"
            }),
        };
        let light_state = HaEntityState {
            state: "unavailable".into(),
            attributes: serde_json::json!({}),
        };

        let snapshot =
            snapshot_from_loaded_states(Some(&pc_state), Some(&ac_state), Some(&light_state));

        assert!(snapshot.connected);
        assert!(!snapshot.ac.is_available);
        assert!(!snapshot.ac_available);
        assert!(!snapshot.ac.is_on);
        assert!(!snapshot.light.is_available);
        assert!(!snapshot.light_available);
        assert!(!snapshot.light_on);
    }

    #[test]
    fn snapshot_from_home_assistant_parses_required_entities() {
        let pc_state = serde_json::json!({"state": "on", "attributes": {}});
        let ac_state = serde_json::json!({"state": "cool", "attributes": {"temperature": 24}});
        let light_state = serde_json::json!({"state": "off", "attributes": {}});

        let snapshot = snapshot_from_home_assistant(&pc_state, &ac_state, &light_state)
            .expect("snapshot should build");

        assert!(snapshot.connected);
        assert!(snapshot.ac.is_available);
        assert!(snapshot.ac_available);
        assert!(snapshot.light.is_available);
        assert!(snapshot.light_available);
        assert!(snapshot.ac.is_on);
        assert_eq!(snapshot.ac.temp, 24);
        assert!(!snapshot.light_on);
    }

    #[test]
    fn snapshot_from_optional_home_assistant_keeps_missing_optional_entities_off() {
        let pc_state = serde_json::json!({"state": "on", "attributes": {}});
        let ac_state = serde_json::json!({"state": "cool", "attributes": {"temperature": 24}});

        let snapshot = snapshot_from_optional_home_assistant(&pc_state, Some(&ac_state), None)
            .expect("snapshot should build");

        assert!(snapshot.connected);
        assert!(snapshot.ac.is_available);
        assert!(snapshot.ac_available);
        assert!(!snapshot.light.is_available);
        assert!(!snapshot.light_available);
        assert!(snapshot.ac.is_on);
        assert_eq!(snapshot.ac.temp, 24);
        assert!(!snapshot.light_on);
    }
}
