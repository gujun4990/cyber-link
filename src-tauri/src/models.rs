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

impl AppConfig {
    pub(crate) fn ac_entity_id(&self) -> Option<&str> {
        self.entity_id.as_ref().and_then(|ids| ids.ac.as_deref())
    }

    pub(crate) fn light_entity_id(&self) -> Option<&str> {
        self.entity_id.as_ref().and_then(|ids| ids.light.as_deref())
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

    pub(crate) fn set_light_available(&mut self, available: bool) {
        self.light.is_available = available;
        self.light_available = available;
    }

    pub(crate) fn set_ac_on(&mut self, is_on: bool) {
        self.ac.is_on = is_on;
    }

    pub(crate) fn set_light_on(&mut self, is_on: bool) {
        self.light.is_on = is_on;
        self.light_on = is_on;
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ACState {
    pub is_on: bool,
    pub is_available: bool,
    pub temp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LightState {
    pub is_on: bool,
    pub is_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceSnapshot {
    pub room: String,
    #[serde(rename = "pcId")]
    pub pc_id: String,
    pub ac: ACState,
    pub light: LightState,
    #[serde(rename = "lightOn")]
    pub light_on: bool,
    #[serde(rename = "acAvailable")]
    pub ac_available: bool,
    #[serde(rename = "lightAvailable")]
    pub light_available: bool,
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
    use super::{ACState, DeviceSnapshot, LightState};

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
            light: LightState {
                is_on: false,
                is_available: false,
            },
            light_on: false,
            ac_available: false,
            light_available: false,
            connected: true,
        };

        snapshot.set_ac_available(true);
        snapshot.set_light_available(true);
        snapshot.set_ac_on(true);
        snapshot.set_light_on(true);

        assert!(snapshot.ac.is_available);
        assert!(snapshot.ac_available);
        assert!(snapshot.light.is_available);
        assert!(snapshot.light_available);
        assert!(snapshot.ac.is_on);
        assert!(snapshot.light.is_on);
        assert!(snapshot.light_on);
    }
}
