use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct DeviceIds {
    #[serde(default)]
    pub ac: Option<String>,
    #[serde(default)]
    pub ambient_light: Option<String>,
    #[serde(default)]
    pub main_light: Option<String>,
    #[serde(default)]
    pub door_sign_light: Option<String>,
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

impl AppConfig {
    pub(crate) fn ac_entity_id(&self) -> Option<&str> {
        self.entity_id.as_ref().and_then(|ids| ids.ac.as_deref())
    }

    pub(crate) fn ambient_light_entity_id(&self) -> Option<&str> {
        self.entity_id
            .as_ref()
            .and_then(|ids| ids.ambient_light.as_deref())
    }

    pub(crate) fn main_light_entity_id(&self) -> Option<&str> {
        self.entity_id
            .as_ref()
            .and_then(|ids| ids.main_light.as_deref())
    }

    pub(crate) fn door_sign_light_entity_id(&self) -> Option<&str> {
        self.entity_id
            .as_ref()
            .and_then(|ids| ids.door_sign_light.as_deref())
    }

    pub(crate) fn light_count(&self) -> u8 {
        let mut count = 0;
        if self.ambient_light_entity_id().is_some() {
            count += 1;
        }
        if self.main_light_entity_id().is_some() {
            count += 1;
        }
        if self.door_sign_light_entity_id().is_some() {
            count += 1;
        }
        count
    }

    pub(crate) fn pc_entity_id(&self) -> Option<&str> {
        self.pc_entity_id.as_deref()
    }
}

impl DeviceSnapshot {
    pub(crate) fn set_ac_available(&mut self, available: bool) {
        self.ac.is_available = available;
        self.ac_available = available;
    }

    pub(crate) fn set_ambient_light_available(&mut self, available: bool) {
        self.switch.is_available = available;
        self.switch_available = available;
    }

    pub(crate) fn set_main_light_available(&mut self, available: bool) {
        self.main_light.is_available = available;
        self.main_light_available = available;
    }

    pub(crate) fn set_door_sign_light_available(&mut self, available: bool) {
        self.door_sign_light.is_available = available;
        self.door_sign_light_available = available;
    }

    pub(crate) fn set_ac_on(&mut self, is_on: bool) {
        self.ac.is_on = is_on;
    }

    pub(crate) fn set_ambient_light_on(&mut self, is_on: bool) {
        self.switch.is_on = is_on;
        self.switch_on = is_on;
    }

    pub(crate) fn set_main_light_on(&mut self, is_on: bool) {
        self.main_light.is_on = is_on;
        self.main_light_on = is_on;
    }

    pub(crate) fn set_door_sign_light_on(&mut self, is_on: bool) {
        self.door_sign_light.is_on = is_on;
        self.door_sign_light_on = is_on;
    }

    pub(crate) fn sync_ac_state(&mut self, is_available: bool, is_on: bool) {
        self.ac.is_available = is_available;
        self.ac_available = is_available;
        self.ac.is_on = is_on;
    }

    pub(crate) fn sync_ambient_light_state(&mut self, is_available: bool, is_on: bool) {
        self.switch.is_available = is_available;
        self.switch_available = is_available;
        self.switch.is_on = is_on;
        self.switch_on = is_on;
    }

    pub(crate) fn sync_main_light_state(&mut self, is_available: bool, is_on: bool) {
        self.main_light.is_available = is_available;
        self.main_light_available = is_available;
        self.main_light.is_on = is_on;
        self.main_light_on = is_on;
    }

    pub(crate) fn sync_door_sign_light_state(&mut self, is_available: bool, is_on: bool) {
        self.door_sign_light.is_available = is_available;
        self.door_sign_light_available = is_available;
        self.door_sign_light.is_on = is_on;
        self.door_sign_light_on = is_on;
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct ACState {
    pub is_on: bool,
    pub is_available: bool,
    pub temp: i32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SwitchState {
    pub is_on: bool,
    pub is_available: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceSnapshot {
    pub room: String,
    #[serde(rename = "pcId")]
    pub pc_id: String,
    pub ac: ACState,
    pub switch: SwitchState,
    #[serde(rename = "mainLight")]
    pub main_light: SwitchState,
    #[serde(rename = "doorSignLight")]
    pub door_sign_light: SwitchState,
    #[serde(rename = "switchOn")]
    pub switch_on: bool,
    #[serde(rename = "mainLightOn")]
    pub main_light_on: bool,
    #[serde(rename = "doorSignLightOn")]
    pub door_sign_light_on: bool,
    #[serde(rename = "acAvailable")]
    pub ac_available: bool,
    #[serde(rename = "switchAvailable")]
    pub switch_available: bool,
    #[serde(rename = "mainLightAvailable")]
    pub main_light_available: bool,
    #[serde(rename = "doorSignLightAvailable")]
    pub door_sign_light_available: bool,
    #[serde(rename = "lightCount")]
    pub light_count: u8,
    pub connected: bool,
}

#[derive(Debug, Clone)]
pub struct HaRequest {
    pub url: String,
    pub body: serde_json::Value,
}

#[derive(Debug, Clone, Deserialize)]
pub struct HaEntityState {
    pub state: String,
    #[serde(default)]
    pub attributes: serde_json::Value,
}

#[cfg(test)]
mod tests {
    use crate::AppConfig;

    use super::{ACState, DeviceSnapshot, SwitchState};

    #[test]
    fn snapshot_sync_helpers_keep_flat_and_nested_fields_aligned() {
        let mut snapshot = DeviceSnapshot {
            room: "核心-01".into(),
            pc_id: "终端-05".into(),
            ac: ACState {
                is_on: false,
                is_available: false,
                temp: 24,
            },
            switch: SwitchState {
                is_on: false,
                is_available: false,
            },
            main_light: SwitchState {
                is_on: false,
                is_available: false,
            },
            door_sign_light: SwitchState {
                is_on: false,
                is_available: false,
            },
            switch_on: false,
            main_light_on: false,
            door_sign_light_on: false,
            ac_available: false,
            switch_available: false,
            main_light_available: false,
            door_sign_light_available: false,
            light_count: 0,
            connected: true,
        };

        snapshot.set_ac_available(true);
        snapshot.set_ambient_light_available(true);
        snapshot.set_ac_on(true);
        snapshot.set_ambient_light_on(true);

        assert!(snapshot.ac.is_available);
        assert!(snapshot.ac_available);
        assert!(snapshot.switch.is_available);
        assert!(snapshot.switch_available);
        assert!(snapshot.ac.is_on);
        assert!(snapshot.switch.is_on);
        assert!(snapshot.switch_on);
    }

    #[test]
    fn app_config_entity_ids_use_ambient_light_key() {
        let config: AppConfig = serde_json::from_str(
            r#"{
                "ha_url": "https://ha.example.local",
                "token": "secret",
                "pc_entity_id": "input_boolean.pc_05_online",
                "entity_id": {
                    "ac": "climate.office_ac",
                    "ambient_light": "switch.office_light",
                    "main_light": "light.ceiling",
                    "door_sign_light": "switch.door_sign"
                }
            }"#,
        )
        .expect("config should parse");

        assert_eq!(config.ambient_light_entity_id(), Some("switch.office_light"));
        assert_eq!(config.main_light_entity_id(), Some("light.ceiling"));
        assert_eq!(config.door_sign_light_entity_id(), Some("switch.door_sign"));
        assert_eq!(config.light_count(), 3);
    }

    #[test]
    fn app_config_entity_ids_reject_old_switch_key() {
        let config = serde_json::from_str::<AppConfig>(
            r#"{
                "ha_url": "https://ha.example.local",
                "token": "secret",
                "entity_id": {
                    "switch": "switch.office_light"
                }
            }"#,
        );

        assert!(config.is_err());
    }
}
