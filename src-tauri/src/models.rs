use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceIds {
    #[serde(default)]
    pub ac: Option<String>,
    #[serde(default)]
    pub switch: Option<String>,
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

    pub(crate) fn switch_entity_id(&self) -> Option<&str> {
        self.entity_id.as_ref().and_then(|ids| ids.switch.as_deref())
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

    pub(crate) fn set_switch_available(&mut self, available: bool) {
        self.switch.is_available = available;
        self.switch_available = available;
    }

    pub(crate) fn set_ac_on(&mut self, is_on: bool) {
        self.ac.is_on = is_on;
    }

    pub(crate) fn set_switch_on(&mut self, is_on: bool) {
        self.switch.is_on = is_on;
        self.switch_on = is_on;
    }

    pub(crate) fn sync_ac_state(&mut self, is_available: bool, is_on: bool) {
        self.ac.is_available = is_available;
        self.ac_available = is_available;
        self.ac.is_on = is_on;
    }

    pub(crate) fn sync_switch_state(&mut self, is_available: bool, is_on: bool) {
        self.switch.is_available = is_available;
        self.switch_available = is_available;
        self.switch.is_on = is_on;
        self.switch_on = is_on;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ACState {
    pub is_on: bool,
    pub is_available: bool,
    pub temp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SwitchState {
    pub is_on: bool,
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceSnapshot {
    pub room: String,
    #[serde(rename = "pcId")]
    pub pc_id: String,
    pub ac: ACState,
    pub switch: SwitchState,
    #[serde(rename = "switchOn")]
    pub switch_on: bool,
    #[serde(rename = "acAvailable")]
    pub ac_available: bool,
    #[serde(rename = "switchAvailable")]
    pub switch_available: bool,
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
            switch_on: false,
            ac_available: false,
            switch_available: false,
            connected: true,
        };

        snapshot.set_ac_available(true);
        snapshot.set_switch_available(true);
        snapshot.set_ac_on(true);
        snapshot.set_switch_on(true);

        assert!(snapshot.ac.is_available);
        assert!(snapshot.ac_available);
        assert!(snapshot.switch.is_available);
        assert!(snapshot.switch_available);
        assert!(snapshot.ac.is_on);
        assert!(snapshot.switch.is_on);
        assert!(snapshot.switch_on);
    }

    #[test]
    fn app_config_entity_ids_can_store_switch_entities() {
        let config = AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(crate::models::DeviceIds {
                ac: Some("climate.office_ac".into()),
                switch: Some("switch.office_light".into()),
            }),
        };

        assert_eq!(config.switch_entity_id(), Some("switch.office_light"));
    }
}
