use crate::models::{AppConfig, DeviceSnapshot, HaEntityState, SwitchState};
use crate::temperature::{parse_double, parse_temperature_unit, temperature_from_attributes};

pub fn initial_snapshot(light_count: u8) -> DeviceSnapshot {
    DeviceSnapshot {
        room: "核心-01".into(),
        pc_id: "终端-05".into(),
        ac: crate::models::ACState {
            is_on: true,
            is_available: true,
            temp: 16,
            ..Default::default()
        },
        switch: SwitchState {
            is_on: true,
            is_available: true,
        },
        main_light: SwitchState {
            is_on: true,
            is_available: true,
        },
        door_sign_light: SwitchState {
            is_on: true,
            is_available: true,
        },
        ambient_light_on: true,
        main_light_on: true,
        door_sign_light_on: true,
        ac_available: true,
        ambient_light_available: true,
        main_light_available: true,
        door_sign_light_available: true,
        light_count,
        connected: true,
    }
}

pub fn offline_snapshot(_config: &AppConfig) -> DeviceSnapshot {
    let mut snapshot = initial_snapshot(_config.light_count());
    snapshot.connected = false;
    snapshot.set_ac_available(false);
    snapshot.set_ambient_light_available(false);
    snapshot.set_main_light_available(false);
    snapshot.set_door_sign_light_available(false);
    snapshot.set_ac_on(false);
    snapshot.set_ambient_light_on(false);
    snapshot.set_main_light_on(false);
    snapshot.set_door_sign_light_on(false);

    snapshot
}

pub fn snapshot_from_loaded_states(
    light_count: u8,
    pc_state: Option<&HaEntityState>,
    ac_state: Option<&HaEntityState>,
    ambient_light_state: Option<&HaEntityState>,
    main_light_state: Option<&HaEntityState>,
    door_sign_light_state: Option<&HaEntityState>,
) -> DeviceSnapshot {
    let mut snapshot = initial_snapshot(light_count);
    snapshot.connected = pc_state.is_some()
        || ac_state.is_some()
        || ambient_light_state.is_some()
        || main_light_state.is_some()
        || door_sign_light_state.is_some();
    snapshot.set_ac_available(false);
    snapshot.set_ambient_light_available(false);
    snapshot.set_main_light_available(false);
    snapshot.set_door_sign_light_available(false);

    if let Some(ac_state) = ac_state {
        snapshot.ac.min_temp = parse_double(&ac_state.attributes, &["min_temp"]);
        snapshot.ac.max_temp = parse_double(&ac_state.attributes, &["max_temp"]);
        snapshot.ac.target_temp_step = parse_double(
            &ac_state.attributes,
            &["step", "temperature_step", "target_temp_step"],
        );
        snapshot.ac.unit_of_measurement = ac_state
            .attributes
            .get("unit_of_measurement")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string);
        snapshot.ac.temperature_unit = parse_temperature_unit(&ac_state.attributes)
            .map(|unit| match unit {
                crate::temperature::TemperatureUnit::Celsius => "°C".to_string(),
                crate::temperature::TemperatureUnit::Fahrenheit => "°F".to_string(),
            });

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

    if let Some(ambient_light_state) = ambient_light_state {
        let is_available = !ambient_light_state.state.eq_ignore_ascii_case("unavailable");
        snapshot.set_ambient_light_available(is_available);
        snapshot.set_ambient_light_on(
            is_available && ambient_light_state.state.eq_ignore_ascii_case("on"),
        );
    } else {
        snapshot.set_ambient_light_available(false);
        snapshot.set_ambient_light_on(false);
    }

    if let Some(main_light_state) = main_light_state {
        let is_available = !main_light_state.state.eq_ignore_ascii_case("unavailable");
        snapshot.set_main_light_available(is_available);
        snapshot.set_main_light_on(is_available && main_light_state.state.eq_ignore_ascii_case("on"));
    } else {
        snapshot.set_main_light_available(false);
        snapshot.set_main_light_on(false);
    }

    if let Some(door_sign_light_state) = door_sign_light_state {
        let is_available = !door_sign_light_state.state.eq_ignore_ascii_case("unavailable");
        snapshot.set_door_sign_light_available(is_available);
        snapshot.set_door_sign_light_on(
            is_available && door_sign_light_state.state.eq_ignore_ascii_case("on"),
        );
    } else {
        snapshot.set_door_sign_light_available(false);
        snapshot.set_door_sign_light_on(false);
    }

    snapshot
}

fn deserialize_entity_state(value: &serde_json::Value) -> anyhow::Result<HaEntityState> {
    Ok(serde_json::from_value(value.clone())?)
}

fn snapshot_from_json_states(
    light_count: u8,
    pc_state: &serde_json::Value,
    ac_state: Option<&serde_json::Value>,
    ambient_light_state: Option<&serde_json::Value>,
    main_light_state: Option<&serde_json::Value>,
    door_sign_light_state: Option<&serde_json::Value>,
) -> anyhow::Result<DeviceSnapshot> {
    let pc_state = deserialize_entity_state(pc_state)?;
    let ac_state = ac_state.map(deserialize_entity_state).transpose()?;
    let ambient_light_state = ambient_light_state.map(deserialize_entity_state).transpose()?;
    let main_light_state = main_light_state.map(deserialize_entity_state).transpose()?;
    let door_sign_light_state = door_sign_light_state.map(deserialize_entity_state).transpose()?;

    Ok(snapshot_from_loaded_states(
        light_count,
        Some(&pc_state),
        ac_state.as_ref(),
        ambient_light_state.as_ref(),
        main_light_state.as_ref(),
        door_sign_light_state.as_ref(),
    ))
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn snapshot_from_home_assistant(
    light_count: u8,
    pc_state: &serde_json::Value,
    ac_state: &serde_json::Value,
    ambient_light_state: &serde_json::Value,
    main_light_state: &serde_json::Value,
    door_sign_light_state: &serde_json::Value,
) -> anyhow::Result<DeviceSnapshot> {
    snapshot_from_json_states(
        light_count,
        pc_state,
        Some(ac_state),
        Some(ambient_light_state),
        Some(main_light_state),
        Some(door_sign_light_state),
    )
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn snapshot_from_optional_home_assistant(
    light_count: u8,
    pc_state: &serde_json::Value,
    ac_state: Option<&serde_json::Value>,
    ambient_light_state: Option<&serde_json::Value>,
    main_light_state: Option<&serde_json::Value>,
    door_sign_light_state: Option<&serde_json::Value>,
) -> anyhow::Result<DeviceSnapshot> {
    snapshot_from_json_states(
        light_count,
        pc_state,
        ac_state,
        ambient_light_state,
        main_light_state,
        door_sign_light_state,
    )
}

#[cfg(test)]
mod tests {
    use super::{
        offline_snapshot, snapshot_from_home_assistant, snapshot_from_loaded_states,
        snapshot_from_optional_home_assistant,
    };
    use crate::models::{AppConfig, DeviceIds, HaEntityState};

    fn sample_config(ac: Option<&str>, switch: Option<&str>) -> AppConfig {
        AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(DeviceIds {
                ac: ac.map(str::to_string),
                ambient_light: switch.map(str::to_string),
                ..Default::default()
            }),
        }
    }

    #[test]
    fn offline_snapshot_keeps_configured_devices_off() {
        let snapshot = offline_snapshot(&sample_config(
            Some("climate.office_ac"),
            Some("switch.office_light"),
        ));

        assert!(!snapshot.connected);
        assert!(!snapshot.ac.is_available);
        assert!(!snapshot.switch.is_available);
        assert!(!snapshot.ac_available);
        assert!(!snapshot.ambient_light_available);
        assert!(!snapshot.ac.is_on);
        assert!(!snapshot.switch.is_on);
        assert!(!snapshot.ambient_light_on);
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

        let snapshot = snapshot_from_loaded_states(0, Some(&pc_state), Some(&ac_state), None, None, None);

        assert!(snapshot.connected);
        assert!(snapshot.ac.is_available);
        assert!(snapshot.ac_available);
        assert!(!snapshot.switch.is_available);
        assert!(!snapshot.ambient_light_available);
        assert!(snapshot.ac.is_on);
        assert_eq!(snapshot.ac.temp, 25);
        assert!(!snapshot.ambient_light_on);
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

        let snapshot = snapshot_from_loaded_states(0, Some(&pc_state), Some(&ac_state), None, None, None);

        assert!(snapshot.ac.is_available);
        assert_eq!(snapshot.ac.temp, 25);
    }

    #[test]
    fn snapshot_from_loaded_states_preserves_unit_of_measurement_metadata() {
        let pc_state = HaEntityState {
            state: "on".into(),
            attributes: serde_json::json!({}),
        };
        let ac_state = HaEntityState {
            state: "cool".into(),
            attributes: serde_json::json!({
                "temperature": 77,
                "unit_of_measurement": "°F"
            }),
        };

        let snapshot = snapshot_from_loaded_states(0, Some(&pc_state), Some(&ac_state), None, None, None);

        assert_eq!(snapshot.ac.unit_of_measurement.as_deref(), Some("°F"));
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
        let ambient_light_state = HaEntityState {
            state: "unavailable".into(),
            attributes: serde_json::json!({}),
        };

        let snapshot =
            snapshot_from_loaded_states(
                0,
                Some(&pc_state),
                Some(&ac_state),
                Some(&ambient_light_state),
                None,
                None,
            );

        assert!(snapshot.connected);
        assert!(!snapshot.ac.is_available);
        assert!(!snapshot.ac_available);
        assert!(!snapshot.ac.is_on);
        assert!(!snapshot.switch.is_available);
        assert!(!snapshot.ambient_light_available);
        assert!(!snapshot.ambient_light_on);
    }

    #[test]
    fn snapshot_from_home_assistant_parses_required_entities() {
        let pc_state = serde_json::json!({"state": "on", "attributes": {}});
        let ac_state = serde_json::json!({"state": "cool", "attributes": {"temperature": 24}});
        let ambient_light_state = serde_json::json!({"state": "off", "attributes": {}});

        let snapshot = snapshot_from_home_assistant(
            1,
            &pc_state,
            &ac_state,
            &ambient_light_state,
            &serde_json::json!({"state": "on", "attributes": {}}),
            &serde_json::json!({"state": "off", "attributes": {}}),
        )
            .expect("snapshot should build");

        assert!(snapshot.connected);
        assert!(snapshot.ac.is_available);
        assert!(snapshot.ac_available);
        assert!(snapshot.switch.is_available);
        assert!(snapshot.ambient_light_available);
        assert!(snapshot.ac.is_on);
        assert_eq!(snapshot.ac.temp, 24);
        assert!(!snapshot.ambient_light_on);
    }

    #[test]
    fn snapshot_from_optional_home_assistant_keeps_missing_optional_entities_off() {
        let pc_state = serde_json::json!({"state": "on", "attributes": {}});
        let ac_state = serde_json::json!({"state": "cool", "attributes": {"temperature": 24}});

        let snapshot = snapshot_from_optional_home_assistant(
            1,
            &pc_state,
            Some(&ac_state),
            None,
            None,
            None,
        )
            .expect("snapshot should build");

        assert!(snapshot.connected);
        assert!(snapshot.ac.is_available);
        assert!(snapshot.ac_available);
        assert!(!snapshot.switch.is_available);
        assert!(!snapshot.ambient_light_available);
        assert!(snapshot.ac.is_on);
        assert_eq!(snapshot.ac.temp, 24);
        assert!(!snapshot.ambient_light_on);
    }
}
