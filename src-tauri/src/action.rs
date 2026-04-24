#[cfg(test)]
mod tests {
    use crate::models::{ACState, AppConfig, DeviceIds, DeviceSnapshot, SwitchState};
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::{apply_action, send_startup_online, ActionArgs, ActionKind, ActionTarget};

    fn sample_snapshot(ac_on: bool, ambient_light_on: bool) -> DeviceSnapshot {
        DeviceSnapshot {
            room: "核心-01".into(),
            pc_id: "终端-05".into(),
            ac: ACState {
                is_on: ac_on,
                is_available: true,
                temp: 16,
            },
            switch: SwitchState {
                is_on: ambient_light_on,
                is_available: true,
            },
            ambient_light_on,
            main_light: SwitchState {
                is_on: false,
                is_available: true,
            },
            door_sign_light: SwitchState {
                is_on: false,
                is_available: true,
            },
            main_light_on: false,
            door_sign_light_on: false,
            ac_available: true,
            ambient_light_available: true,
            main_light_available: true,
            door_sign_light_available: true,
            light_count: 3,
            connected: true,
        }
    }

    fn disable_proxy_env() {
        std::env::set_var("NO_PROXY", "127.0.0.1,localhost");
        std::env::set_var("no_proxy", "127.0.0.1,localhost");
        std::env::set_var("HTTP_PROXY", "");
        std::env::set_var("HTTPS_PROXY", "");
        std::env::set_var("http_proxy", "");
        std::env::set_var("https_proxy", "");
    }

    async fn respond_with_request_assertion(
        mut socket: tokio::net::TcpStream,
        expected_substring: &'static str,
        body: &'static str,
    ) {
        let mut buf = vec![0u8; 4096];
        let len = socket.read(&mut buf).await.expect("read request");
        let request = String::from_utf8_lossy(&buf[..len]);
        assert!(
            request.contains(expected_substring),
            "request did not contain expected substring {expected_substring:?}: {request}"
        );
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write response");
    }

    async fn respond_with_json(mut socket: tokio::net::TcpStream, body: &'static str) {
        let mut buf = vec![0u8; 4096];
        let _ = socket.read(&mut buf).await.expect("read request");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write response");
    }

    async fn respond_with_error(mut socket: tokio::net::TcpStream, status: u16, body: &'static str) {
        let mut buf = vec![0u8; 4096];
        let _ = socket.read(&mut buf).await.expect("read request");
        let response = format!(
            "HTTP/1.1 {status} ERROR\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        socket
            .write_all(response.as_bytes())
            .await
            .expect("write response");
    }

    #[tokio::test]
    async fn ac_toggle_returns_immediately_without_retry() {
        disable_proxy_env();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept post");
            respond_with_json(socket, r#"{"state":"ok","attributes":{}}"#).await;
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: None,
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                ambient_light: None,
                ..Default::default()
            }),
        };

        let outcome = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            apply_action(
                &config,
                sample_snapshot(false, false),
                ActionArgs {
                    action: ActionKind::AcToggle,
                    target: None,
                    value: None,
                },
            ),
        )
        .await
        .expect("action should return immediately")
        .expect("action should succeed");

        server.await.expect("server task");

        assert!(outcome.error.is_none());
        assert!(outcome.snapshot.ac.is_on);
        assert_eq!(outcome.snapshot.ac.temp, 16);
    }

    #[tokio::test]
    async fn ac_toggle_preserves_snapshot_when_request_fails() {
        disable_proxy_env();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept request");
            respond_with_error(socket, 500, r#"{"message":"boom"}"#).await;
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: None,
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                ambient_light: None,
                ..Default::default()
            }),
        };

        let original = sample_snapshot(false, false);
        let outcome = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            apply_action(
                &config,
                original.clone(),
                ActionArgs {
                    action: ActionKind::AcToggle,
                    target: None,
                    value: None,
                },
            ),
        )
        .await
        .expect("action should return immediately")
        .expect("action should succeed");

        server.await.expect("server task");

        assert!(outcome.error.is_some());
        assert_eq!(outcome.snapshot.ac.is_on, original.ac.is_on);
        assert_eq!(outcome.snapshot.ac.temp, original.ac.temp);
    }

    #[tokio::test]
    async fn switch_toggle_returns_immediately_without_retry() {
        disable_proxy_env();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept request");
            respond_with_json(socket, r#"{"state":"ok","attributes":{}}"#).await;
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: None,
            entity_id: Some(DeviceIds {
                ac: None,
                ambient_light: Some("switch.office_light".into()),
                ..Default::default()
            }),
        };

        let outcome = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            apply_action(
                &config,
                sample_snapshot(false, false),
                ActionArgs {
                    action: ActionKind::SwitchToggle,
                    target: Some(ActionTarget::AmbientLight),
                    value: None,
                },
            ),
        )
        .await
        .expect("action should return immediately")
        .expect("action should succeed");

        server.await.expect("server task");

        assert!(outcome.error.is_none());
        assert!(outcome.snapshot.ambient_light_on);
    }

    #[tokio::test]
    async fn switch_toggle_routes_to_the_requested_light_target() {
        disable_proxy_env();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = tokio::spawn(async move {
            let mut seen = 0usize;
            loop {
                let accepted =
                    tokio::time::timeout(std::time::Duration::from_secs(5), listener.accept())
                        .await;

                let Ok(Ok((socket, _))) = accepted else {
                    break;
                };

                match seen {
                    0 => respond_with_json(socket, r#"{"state":"ok","attributes":{}}"#).await,
                    1 => {
                        respond_with_json(socket, r#"{"state":"on","attributes":{}}"#).await
                    }
                    _ => respond_with_json(socket, r#"{"state":"on","attributes":{}}"#).await,
                }

                seen += 1;
            }
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: None,
            entity_id: Some(DeviceIds {
                ac: None,
                ambient_light: None,
                main_light: Some("light.ceiling".into()),
                ..Default::default()
            }),
        };

        let outcome = apply_action(
            &config,
            sample_snapshot(false, false),
            ActionArgs {
                action: ActionKind::SwitchToggle,
                target: Some(ActionTarget::MainLight),
                value: None,
            },
        )
        .await
        .expect("action should refresh state");

        server.await.expect("server task");

        assert!(outcome.error.is_none());
        assert!(outcome.snapshot.main_light_on);
        assert!(!outcome.snapshot.ambient_light_on);
    }

    #[tokio::test]
    async fn ac_set_temp_returns_normalized_temperature_without_retry() {
        disable_proxy_env();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept normalize state");
            respond_with_json(
                socket,
                r#"{"state":"cool","attributes":{"temperature":24,"min_temp":60,"max_temp":80,"target_temp_step":1,"temperature_unit":"F"}}"#,
            )
            .await;

            let (socket, _) = listener.accept().await.expect("accept post");
            respond_with_json(socket, r#"{"state":"ok","attributes":{}}"#).await;
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: None,
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                ambient_light: None,
                ..Default::default()
            }),
        };

        let outcome = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            apply_action(
                &config,
                sample_snapshot(true, false),
                ActionArgs {
                    action: ActionKind::AcSetTemp,
                    target: None,
                    value: Some(26),
                },
            ),
        )
        .await
        .expect("action should return immediately")
        .expect("action should succeed");

        server.await.expect("server task");

        assert!(outcome.error.is_none());
        assert_eq!(outcome.snapshot.ac.temp, 26);
    }

    #[tokio::test]
    async fn ac_set_temp_returns_clamped_temperature_without_retry() {
        disable_proxy_env();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept normalize state");
            respond_with_json(
                socket,
                r#"{"state":"cool","attributes":{"temperature":24,"min_temp":16,"max_temp":30,"target_temp_step":1,"temperature_unit":"°C"}}"#,
            )
            .await;

            let (socket, _) = listener.accept().await.expect("accept post");
            respond_with_json(socket, r#"{"state":"ok","attributes":{}}"#).await;
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: None,
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                ambient_light: None,
                ..Default::default()
            }),
        };

        let outcome = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            apply_action(
                &config,
                sample_snapshot(true, false),
                ActionArgs {
                    action: ActionKind::AcSetTemp,
                    target: None,
                    value: Some(31),
                },
            ),
        )
        .await
        .expect("action should return immediately")
        .expect("action should succeed");

        server.await.expect("server task");

        assert!(outcome.error.is_none());
        assert_eq!(outcome.snapshot.ac.temp, 30);
    }

    #[tokio::test]
    async fn ac_set_temp_accepts_slower_ha_response() {
        disable_proxy_env();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept normalize state");
            respond_with_json(
                socket,
                r#"{"state":"cool","attributes":{"temperature":24,"min_temp":16,"max_temp":30,"target_temp_step":1,"temperature_unit":"°C"}}"#,
            )
            .await;

            let (socket, _) = listener.accept().await.expect("accept post");
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            respond_with_json(socket, r#"{"state":"ok","attributes":{}}"#).await;
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: None,
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                ambient_light: None,
                ..Default::default()
            }),
        };

        let outcome = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            apply_action(
                &config,
                sample_snapshot(true, false),
                ActionArgs {
                    action: ActionKind::AcSetTemp,
                    target: None,
                    value: Some(26),
                },
            ),
        )
        .await
        .expect("action should not time out")
        .expect("action should succeed");

        server.await.expect("server task");

        assert!(outcome.error.is_none());
        assert_eq!(outcome.snapshot.ac.temp, 26);
    }

    #[tokio::test]
    async fn startup_with_pc_entity_sends_only_pc_online() {
        disable_proxy_env();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept pc request");
            respond_with_request_assertion(
                socket,
                "/api/services/input_boolean/turn_on",
                r#"{"state":"ok","attributes":{}}"#,
            )
            .await;
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                ambient_light: Some("switch.office_light".into()),
                ..Default::default()
            }),
        };

        let outcome = send_startup_online(&config).await;

        server.await.expect("server task");

        assert!(outcome.is_ok());
    }

    #[tokio::test]
    async fn startup_with_pc_entity_only_marks_connected() {
        disable_proxy_env();

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept pc request");
            respond_with_json(socket, r#"{"state":"ok","attributes":{}}"#).await;
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                ambient_light: Some("switch.office_light".into()),
                ..Default::default()
            }),
        };

        let mut snapshot = sample_snapshot(false, false);
        snapshot.connected = false;
        snapshot.ac_available = false;
        snapshot.ambient_light_available = false;
        snapshot.ac.is_available = false;
        snapshot.switch.is_available = false;
        snapshot.ac.is_on = false;
        snapshot.ambient_light_on = false;
        snapshot.switch.is_on = false;

        let outcome = apply_action(
            &config,
            snapshot,
            ActionArgs {
                action: ActionKind::StartupOnline,
                target: None,
                value: None,
            },
        )
        .await
        .expect("startup online should succeed");

        server.await.expect("server task");

        assert!(outcome.error.is_none());
        assert!(outcome.snapshot.connected);
        assert!(!outcome.snapshot.ac_available);
        assert!(!outcome.snapshot.ambient_light_available);
        assert!(!outcome.snapshot.ac.is_available);
        assert!(!outcome.snapshot.switch.is_available);
        assert!(!outcome.snapshot.ac.is_on);
        assert!(!outcome.snapshot.ambient_light_on);
        assert!(!outcome.snapshot.switch.is_on);
    }

    #[tokio::test]
    async fn startup_without_pc_entity_keeps_device_sync() {
        disable_proxy_env();

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept ac request");
            respond_with_json(socket, r#"{"state":"ok","attributes":{}}"#).await;

            let (socket, _) = listener.accept().await.expect("accept switch request");
            respond_with_json(socket, r#"{"state":"ok","attributes":{}}"#).await;
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: None,
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                ambient_light: Some("switch.office_light".into()),
                ..Default::default()
            }),
        };

        let mut snapshot = sample_snapshot(false, false);
        snapshot.connected = false;

        let outcome = apply_action(
            &config,
            snapshot,
            ActionArgs {
                action: ActionKind::StartupOnline,
                target: None,
                value: None,
            },
        )
        .await
        .expect("startup online should succeed");

        server.await.expect("server task");

        assert!(outcome.snapshot.connected);
        assert!(outcome.snapshot.ac_available);
        assert!(outcome.snapshot.ac.is_available);
        assert!(outcome.snapshot.ambient_light_available);
        assert!(outcome.snapshot.switch.is_available);
        assert!(outcome.snapshot.ac.is_on);
        assert!(outcome.snapshot.ambient_light_on);
        assert!(outcome.snapshot.switch.is_on);
    }

    #[tokio::test]
    async fn startup_online_restores_connected_state_from_offline_snapshot() {
        disable_proxy_env();

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept ac request");
            respond_with_json(socket, r#"{"state":"ok","attributes":{}}"#).await;

            let (socket, _) = listener.accept().await.expect("accept switch request");
            respond_with_json(socket, r#"{"state":"ok","attributes":{}}"#).await;
        });

        let config = AppConfig {
            ha_url: format!("http://{}", addr),
            token: "secret".into(),
            pc_entity_id: None,
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                ambient_light: Some("switch.office_light".into()),
                ..Default::default()
            }),
        };

        let mut snapshot = sample_snapshot(false, false);
        snapshot.connected = false;

        let outcome = apply_action(
            &config,
            snapshot,
            ActionArgs {
                action: ActionKind::StartupOnline,
                target: None,
                value: None,
            },
        )
        .await
        .expect("startup online should succeed");

        server.await.expect("server task");

        assert!(outcome.snapshot.connected);
    }

    #[tokio::test]
    async fn startup_online_does_not_fabricate_connection_without_targets() {
        let config = AppConfig {
            ha_url: "http://127.0.0.1:9".into(),
            token: "secret".into(),
            pc_entity_id: None,
            entity_id: Some(DeviceIds {
                ac: None,
                ambient_light: None,
                ..Default::default()
            }),
        };

        let mut snapshot = sample_snapshot(false, false);
        snapshot.connected = false;

        let outcome = apply_action(
            &config,
            snapshot,
            ActionArgs {
                action: ActionKind::StartupOnline,
                target: None,
                value: None,
            },
        )
        .await
        .expect("startup online should succeed");

        assert!(!outcome.snapshot.connected);
    }

    #[tokio::test]
    async fn startup_online_with_pc_entity_keeps_device_states_offline() {
        disable_proxy_env();

        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept pc request");
            respond_with_json(socket, r#"{"state":"ok","attributes":{}}"#).await;
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                ambient_light: Some("switch.office_light".into()),
                ..Default::default()
            }),
        };

        let mut snapshot = sample_snapshot(false, false);
        snapshot.connected = false;
        snapshot.ac_available = false;
        snapshot.ambient_light_available = false;
        snapshot.ac.is_available = false;
        snapshot.switch.is_available = false;
        snapshot.ac.is_on = false;
        snapshot.ambient_light_on = false;
        snapshot.switch.is_on = false;

        let outcome = apply_action(
            &config,
            snapshot,
            ActionArgs {
                action: ActionKind::StartupOnline,
                target: None,
                value: None,
            },
        )
        .await
        .expect("startup online should succeed");

        server.await.expect("server task");

        assert!(outcome.error.is_none());
        assert!(outcome.snapshot.connected);
        assert!(!outcome.snapshot.ac_available);
        assert!(!outcome.snapshot.ambient_light_available);
        assert!(!outcome.snapshot.ac.is_available);
        assert!(!outcome.snapshot.switch.is_available);
        assert!(!outcome.snapshot.ac.is_on);
        assert!(!outcome.snapshot.ambient_light_on);
        assert!(!outcome.snapshot.switch.is_on);
    }

    #[test]
    fn snapshot_helpers_keep_flat_and_nested_states_in_sync() {
        let mut snapshot = sample_snapshot(false, false);

        snapshot.sync_ac_state(true, true);
        snapshot.sync_ambient_light_state(true, false);
        snapshot.sync_main_light_state(true, true);
        snapshot.sync_door_sign_light_state(false, false);

        assert!(snapshot.ac_available);
        assert!(snapshot.ac.is_available);
        assert!(snapshot.ac.is_on);
        assert!(snapshot.ambient_light_available);
        assert!(snapshot.switch.is_available);
        assert!(!snapshot.ambient_light_on);
        assert!(!snapshot.switch.is_on);
        assert!(snapshot.main_light_available);
        assert!(snapshot.main_light.is_available);
        assert!(snapshot.main_light_on);
        assert!(snapshot.main_light.is_on);
        assert!(!snapshot.door_sign_light_available);
        assert!(!snapshot.door_sign_light.is_available);
    }

    #[test]
    fn action_kind_accepts_switch_toggle_payload_only() {
        let switch_action: ActionKind = serde_json::from_str(r#""switch_toggle""#).expect("action");

        assert_eq!(switch_action, ActionKind::SwitchToggle);
        assert!(serde_json::from_str::<ActionKind>(r#""main_light_toggle""#).is_err());
        assert!(serde_json::from_str::<ActionKind>(r#""door_sign_light_toggle""#).is_err());
    }

    #[test]
    fn action_args_parse_switch_toggle_target() {
        let args: ActionArgs = serde_json::from_str(
            r#"{"action":"switch_toggle","target":"mainLight"}"#,
        )
        .expect("args");

        assert_eq!(args.action, ActionKind::SwitchToggle);
        assert_eq!(args.target, Some(ActionTarget::MainLight));
    }
}

use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

use crate::ha_client::{
    climate_set_temperature_request, climate_temperature_targets, climate_turn_off_request,
    climate_turn_on_request, entity_turn_off_request, entity_turn_on_request,
    fetch_ha_entity_state, send_ha_request, send_ha_request_with_timeout,
};
use crate::models::{AppConfig, DeviceSnapshot, HaRequest};
use crate::snapshot::snapshot_from_loaded_states;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ActionArgs {
    pub(crate) action: ActionKind,
    pub(crate) target: Option<ActionTarget>,
    pub(crate) value: Option<i32>,
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) enum ActionTarget {
    AmbientLight,
    MainLight,
    DoorSignLight,
}

#[derive(Debug)]
pub(crate) struct ActionApplyOutcome {
    pub(crate) snapshot: DeviceSnapshot,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
enum HaAction {
    ToggleAc { on: bool },
    ToggleEntity { entity_id: String, on: bool },
}

impl HaAction {
    pub fn into_request(&self, config: &AppConfig) -> Result<HaRequest> {
        match self {
            Self::ToggleAc { on } => {
                if *on {
                    climate_turn_on_request(config)
                } else {
                    climate_turn_off_request(config)
                }
            }
            Self::ToggleEntity { entity_id, on } => {
                if *on {
                    entity_turn_on_request(config, entity_id)
                } else {
                    entity_turn_off_request(config, entity_id)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ActionKind {
    AcToggle,
    AcSetTemp,
    SwitchToggle,
    StartupOnline,
    ShutdownSignal,
}

fn build_notification_request(config: &AppConfig, state: bool) -> Result<HaRequest> {
    let pc_entity_id = config
        .pc_entity_id()
        .ok_or_else(|| anyhow!("pc entity is not configured"))?;
    let base = config.ha_url.trim_end_matches('/');
    Ok(HaRequest {
        url: format!(
            "{base}/api/services/input_boolean/{}",
            if state { "turn_on" } else { "turn_off" }
        ),
        body: json!({"entity_id": pc_entity_id}),
    })
}

fn notification_timeout(state: bool) -> Duration {
    if state {
        Duration::from_secs(2)
    } else {
        Duration::from_millis(900)
    }
}

async fn send_ha_action(config: &AppConfig, action: HaAction) -> Result<()> {
    send_ha_request(config, action.into_request(config)?).await
}

async fn send_entity_toggle_request(config: &AppConfig, entity_id: &str, on: bool) -> Result<()> {
    send_ha_action(
        config,
        HaAction::ToggleEntity {
            entity_id: entity_id.to_string(),
            on,
        },
    )
    .await
}

fn configured_light_entity_ids(config: &AppConfig) -> [Option<&str>; 3] {
    [
        config.ambient_light_entity_id(),
        config.main_light_entity_id(),
        config.door_sign_light_entity_id(),
    ]
}

async fn send_ha_notification(config: &AppConfig, state: bool) -> Result<()> {
    let request = build_notification_request(config, state)?;
    send_ha_request_with_timeout(config, request, notification_timeout(state)).await
}

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
    snapshot.sync_door_sign_light_state(
        door_sign_light_available,
        door_sign_light_available,
    );
}

pub(crate) async fn send_startup_online(config: &AppConfig) -> Result<()> {
    if config.pc_entity_id().is_some() {
        send_ha_notification(config, true).await?;
        return Ok(());
    }

    let mut first_err: Option<anyhow::Error> = None;
    if config.ac_entity_id().is_some() {
        if let Err(err) = send_ha_action(config, HaAction::ToggleAc { on: true }).await {
            first_err.get_or_insert(err);
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

pub(crate) async fn fetch_current_snapshot(config: &AppConfig) -> Result<DeviceSnapshot> {
    let ac_state = match config.ac_entity_id() {
        Some(entity_id) => Some(fetch_ha_entity_state(config, entity_id).await?),
        None => None,
    };
    let ambient_light_state = match config.ambient_light_entity_id() {
        Some(entity_id) => Some(fetch_ha_entity_state(config, entity_id).await?),
        None => None,
    };
    let main_light_state = match config.main_light_entity_id() {
        Some(entity_id) => Some(fetch_ha_entity_state(config, entity_id).await?),
        None => None,
    };
    let door_sign_light_state = match config.door_sign_light_entity_id() {
        Some(entity_id) => Some(fetch_ha_entity_state(config, entity_id).await?),
        None => None,
    };
    let pc_state = match config.pc_entity_id() {
        Some(entity_id) => Some(fetch_ha_entity_state(config, entity_id).await?),
        None => None,
    };

    Ok(snapshot_from_loaded_states(
        config.light_count(),
        pc_state.as_ref(),
        ac_state.as_ref(),
        ambient_light_state.as_ref(),
        main_light_state.as_ref(),
        door_sign_light_state.as_ref(),
    ))
}

pub(crate) async fn apply_action(
    config: &AppConfig,
    snapshot: DeviceSnapshot,
    args: ActionArgs,
) -> Result<ActionApplyOutcome> {
    let mut snapshot = snapshot;
    match args.action {
        ActionKind::AcToggle => {
            if config.ac_entity_id().is_none() {
                return Ok(ActionApplyOutcome {
                    snapshot,
                    error: None,
                });
            }
            let original = snapshot.clone();
            let next = !snapshot.ac.is_on;
            let result = send_ha_request(
                config,
                HaAction::ToggleAc { on: next }.into_request(config)?,
            )
            .await;
            if let Err(err) = result {
                return Ok(ActionApplyOutcome {
                    snapshot: original,
                    error: Some(err.to_string()),
                });
            }
            snapshot.ac.is_on = next;
            return Ok(ActionApplyOutcome {
                snapshot,
                error: None,
            });
        }
        ActionKind::AcSetTemp => {
            if config.ac_entity_id().is_none() {
                return Ok(ActionApplyOutcome {
                    snapshot,
                    error: None,
                });
            }
            let original = snapshot.clone();
            let temp_celsius = args.value.ok_or_else(|| anyhow!("missing temperature"))?;
            let (normalized_temp, confirmed_temp) =
                climate_temperature_targets(config, temp_celsius).await?;
            let request = climate_set_temperature_request(config, normalized_temp)?;
            let result = send_ha_request_with_timeout(config, request, Duration::from_secs(10)).await;
            if let Err(err) = result {
                return Ok(ActionApplyOutcome {
                    snapshot: original,
                    error: Some(err.to_string()),
                });
            }
            snapshot.ac.is_on = true;
            snapshot.ac.temp = confirmed_temp;
            return Ok(ActionApplyOutcome {
                snapshot,
                error: None,
            });
        }
        ActionKind::SwitchToggle => {
            let target = args.target.ok_or_else(|| anyhow!("missing light target"))?;
            let entity_id = match target {
                ActionTarget::AmbientLight => config.ambient_light_entity_id(),
                ActionTarget::MainLight => config.main_light_entity_id(),
                ActionTarget::DoorSignLight => config.door_sign_light_entity_id(),
            }
            .ok_or_else(|| anyhow!("light target is not configured"))?;
            let original = snapshot.clone();
            let next = match target {
                ActionTarget::AmbientLight => !snapshot.ambient_light_on,
                ActionTarget::MainLight => !snapshot.main_light_on,
                ActionTarget::DoorSignLight => !snapshot.door_sign_light_on,
            };
            let result = send_entity_toggle_request(config, entity_id, next).await;
            if let Err(err) = result {
                return Ok(ActionApplyOutcome {
                    snapshot: original,
                    error: Some(err.to_string()),
                });
            }
            match target {
                ActionTarget::AmbientLight => {
                    snapshot.sync_ambient_light_state(snapshot.ambient_light_available, next)
                }
                ActionTarget::MainLight => snapshot.sync_main_light_state(snapshot.main_light_available, next),
                ActionTarget::DoorSignLight => {
                    snapshot.sync_door_sign_light_state(snapshot.door_sign_light_available, next)
                }
            }
            return Ok(ActionApplyOutcome {
                snapshot,
                error: None,
            });
        }
        ActionKind::StartupOnline => {
            send_startup_online(config).await?;
            apply_startup_snapshot(&mut snapshot, config);
        }
        ActionKind::ShutdownSignal => {
            send_shutdown_signal(config).await?;
        }
    }

    Ok(ActionApplyOutcome {
        snapshot,
        error: None,
    })
}
