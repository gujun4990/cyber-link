#[cfg(test)]
mod tests {
    use crate::models::{ACState, AppConfig, DeviceIds, DeviceSnapshot, LightState};
    use std::sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    use super::{apply_action, ActionArgs, ActionKind};

    fn sample_snapshot(ac_on: bool, light_on: bool) -> DeviceSnapshot {
        DeviceSnapshot {
            room: "核心-01".into(),
            pc_id: "终端-05".into(),
            ac: ACState {
                is_on: ac_on,
                is_available: true,
                temp: 16,
            },
            light: LightState {
                is_on: light_on,
                is_available: true,
            },
            light_on,
            ac_available: true,
            light_available: true,
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

    #[tokio::test]
    async fn ac_toggle_confirms_refreshed_snapshot() {
        disable_proxy_env();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");
        let ac_is_on = Arc::new(AtomicBool::new(false));
        let ac_is_on_for_server = Arc::clone(&ac_is_on);

        let server = tokio::spawn(async move {
            let (socket, _) = listener.accept().await.expect("accept post");
            respond_with_json(socket, r#"{"state":"ok","attributes":{}}"#).await;
            ac_is_on_for_server.store(true, Ordering::SeqCst);

            let (socket, _) = listener.accept().await.expect("accept refresh");
            let body = if ac_is_on_for_server.load(Ordering::SeqCst) {
                r#"{"state":"cool","attributes":{"temperature":24}}"#
            } else {
                r#"{"state":"off","attributes":{}}"#
            };
            respond_with_json(socket, body).await;
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: None,
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: None,
            }),
        };

        let outcome = apply_action(
            &config,
            sample_snapshot(false, false),
            ActionArgs {
                action: ActionKind::AcToggle,
                value: None,
            },
        )
        .await
        .expect("action should refresh state");

        server.await.expect("server task");

        assert!(outcome.error.is_none());
        assert!(outcome.snapshot.ac.is_on);
        assert_eq!(outcome.snapshot.ac.temp, 24);
    }

    #[tokio::test]
    async fn light_toggle_confirms_refreshed_snapshot() {
        disable_proxy_env();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");
        let light_is_on = Arc::new(AtomicBool::new(false));
        let light_is_on_for_server = Arc::clone(&light_is_on);

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
                    0 => {
                        respond_with_json(socket, r#"{"state":"ok","attributes":{}}"#).await;
                        light_is_on_for_server.store(true, Ordering::SeqCst);
                    }
                    1 => {
                        let body = if light_is_on_for_server.load(Ordering::SeqCst) {
                            r#"{"state":"on","attributes":{}}"#
                        } else {
                            r#"{"state":"off","attributes":{}}"#
                        };
                        respond_with_json(socket, body).await;
                    }
                    2 => {
                        respond_with_json(socket, r#"{"state":"on","attributes":{}}"#).await;
                    }
                    _ => {
                        respond_with_json(socket, r#"{"state":"on","attributes":{}}"#).await;
                    }
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
                light: Some("light.office_light".into()),
            }),
        };

        let outcome = apply_action(
            &config,
            sample_snapshot(false, false),
            ActionArgs {
                action: ActionKind::LightToggle,
                value: None,
            },
        )
        .await
        .expect("action should refresh state");

        server.await.expect("server task");

        assert!(outcome.error.is_none());
        assert!(outcome.snapshot.light_on);
    }

    #[tokio::test]
    async fn ac_set_temp_confirms_normalized_temperature() {
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

            let (socket, _) = listener.accept().await.expect("accept refresh");
            respond_with_json(
                socket,
                r#"{"state":"cool","attributes":{"temperature":79}}"#,
            )
            .await;
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: None,
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: None,
            }),
        };

        let outcome = apply_action(
            &config,
            sample_snapshot(true, false),
            ActionArgs {
                action: ActionKind::AcSetTemp,
                value: Some(26),
            },
        )
        .await
        .expect("action should refresh state");

        server.await.expect("server task");

        assert!(outcome.error.is_none());
        assert_eq!(outcome.snapshot.ac.temp, 26);
    }

    #[tokio::test]
    async fn ac_set_temp_confirms_clamped_temperature() {
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

            let (socket, _) = listener.accept().await.expect("accept refresh");
            respond_with_json(
                socket,
                r#"{"state":"cool","attributes":{"temperature":30}}"#,
            )
            .await;
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: None,
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: None,
            }),
        };

        let outcome = apply_action(
            &config,
            sample_snapshot(true, false),
            ActionArgs {
                action: ActionKind::AcSetTemp,
                value: Some(31),
            },
        )
        .await
        .expect("action should refresh state");

        server.await.expect("server task");

        assert!(outcome.error.is_none());
        assert_eq!(outcome.snapshot.ac.temp, 30);
    }

    #[tokio::test]
    async fn startup_online_keeps_device_availability_and_turn_on_behavior() {
        disable_proxy_env();
        let listener = TcpListener::bind("127.0.0.1:0").await.expect("listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = tokio::spawn(async move {
            for _ in 0..3 {
                let (socket, _) = listener.accept().await.expect("accept request");
                respond_with_json(socket, r#"{"state":"ok","attributes":{}}"#).await;
            }
        });

        let config = AppConfig {
            ha_url: format!("http://{addr}"),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                light: Some("light.office_light".into()),
            }),
        };

        let outcome = apply_action(
            &config,
            sample_snapshot(false, false),
            ActionArgs {
                action: ActionKind::StartupOnline,
                value: None,
            },
        )
        .await
        .expect("startup online should succeed");

        server.await.expect("server task");

        assert!(outcome.error.is_none());
        assert!(outcome.snapshot.ac_available);
        assert!(outcome.snapshot.light_available);
        assert!(outcome.snapshot.ac.is_on);
        assert!(outcome.snapshot.light_on);
    }
}

use anyhow::{anyhow, Result};
use serde::Deserialize;
use serde_json::json;
use std::time::Duration;

use crate::ha_client::{
    climate_set_temperature_request, climate_temperature_targets, climate_turn_off_request,
    climate_turn_on_request, fetch_ha_entity_state, light_turn_off_request, light_turn_on_request,
    send_ha_request, send_ha_request_with_timeout,
};
use crate::models::{AppConfig, DeviceSnapshot, HaRequest};
use crate::snapshot::snapshot_from_loaded_states;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct ActionArgs {
    pub(crate) action: ActionKind,
    pub(crate) value: Option<i32>,
}

#[derive(Debug)]
pub(crate) struct ActionApplyOutcome {
    pub(crate) snapshot: DeviceSnapshot,
    pub(crate) error: Option<String>,
}

#[derive(Debug, Clone)]
enum HaAction {
    ToggleAc { on: bool },
    ToggleLight { on: bool },
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
            Self::ToggleLight { on } => {
                if *on {
                    light_turn_on_request(config)
                } else {
                    light_turn_off_request(config)
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
    LightToggle,
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

const ACTION_CONFIRMATION_INITIAL_DELAY: Duration = Duration::from_secs(2);
const ACTION_CONFIRMATION_RETRY_COUNT: usize = 5;
const ACTION_CONFIRMATION_RETRY_INTERVAL: Duration = Duration::from_secs(2);

async fn confirm_action_snapshot_with_delays<F>(
    config: &AppConfig,
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
        match fetch_current_snapshot(config).await {
            Ok(snapshot) => {
                if expected(&snapshot) {
                    return ActionApplyOutcome {
                        snapshot,
                        error: None,
                    };
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

    ActionApplyOutcome {
        snapshot: last_snapshot,
        error: last_error,
    }
}

async fn send_ha_action(config: &AppConfig, action: HaAction) -> Result<()> {
    send_ha_request(config, action.into_request(config)?).await
}

async fn send_ha_notification(config: &AppConfig, state: bool) -> Result<()> {
    let request = build_notification_request(config, state)?;
    send_ha_request_with_timeout(config, request, notification_timeout(state)).await
}

fn startup_online_targets(config: &AppConfig) -> (bool, bool, bool) {
    (
        config.pc_entity_id().is_some(),
        config.ac_entity_id().is_some(),
        config.light_entity_id().is_some(),
    )
}

pub(crate) async fn send_startup_online(config: &AppConfig) -> Result<()> {
    let mut first_err: Option<anyhow::Error> = None;
    let (send_pc, send_ac, send_light) = startup_online_targets(config);

    if send_pc {
        if let Err(err) = send_ha_notification(config, true).await {
            first_err = Some(err);
        }
    }

    if send_ac {
        if let Err(err) = send_ha_action(config, HaAction::ToggleAc { on: true }).await {
            first_err.get_or_insert(err);
        }
    }

    if send_light {
        if let Err(err) = send_ha_action(config, HaAction::ToggleLight { on: true }).await {
            first_err.get_or_insert(err);
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

    if config.light_entity_id().is_some() {
        if let Err(err) = send_ha_action(config, HaAction::ToggleLight { on: false }).await {
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
    let light_state = match config.light_entity_id() {
        Some(entity_id) => Some(fetch_ha_entity_state(config, entity_id).await?),
        None => None,
    };
    let pc_state = match config.pc_entity_id() {
        Some(entity_id) => Some(fetch_ha_entity_state(config, entity_id).await?),
        None => None,
    };

    Ok(snapshot_from_loaded_states(
        pc_state.as_ref(),
        ac_state.as_ref(),
        light_state.as_ref(),
    ))
}

pub(crate) async fn apply_action(
    config: &AppConfig,
    snapshot: DeviceSnapshot,
    args: ActionArgs,
) -> Result<ActionApplyOutcome> {
    apply_action_with_delays(
        config,
        snapshot,
        args,
        ACTION_CONFIRMATION_INITIAL_DELAY,
        ACTION_CONFIRMATION_RETRY_COUNT,
        ACTION_CONFIRMATION_RETRY_INTERVAL,
    )
    .await
}

async fn apply_action_with_delays(
    config: &AppConfig,
    mut snapshot: DeviceSnapshot,
    args: ActionArgs,
    initial_delay: Duration,
    retry_count: usize,
    retry_interval: Duration,
) -> Result<ActionApplyOutcome> {
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
            snapshot.ac.is_on = next;
            return Ok(confirm_action_snapshot_with_delays(
                config,
                &original,
                result.err().map(|err| err.to_string()),
                move |current| current.ac.is_on == next,
                initial_delay,
                retry_count,
                retry_interval,
                "空调尚未进入预期开关状态，继续等待刷新。",
            )
            .await);
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
            let result = send_ha_request(config, request).await;
            snapshot.ac.temp = confirmed_temp;
            return Ok(confirm_action_snapshot_with_delays(
                config,
                &original,
                result.err().map(|err| err.to_string()),
                move |current| current.ac.is_on && current.ac.temp == confirmed_temp,
                initial_delay,
                retry_count,
                retry_interval,
                "空调尚未进入预期温度状态，继续等待刷新。",
            )
            .await);
        }
        ActionKind::LightToggle => {
            if config.light_entity_id().is_none() {
                return Ok(ActionApplyOutcome {
                    snapshot,
                    error: None,
                });
            }
            let original = snapshot.clone();
            let next = !snapshot.light_on;
            let result = send_ha_request(
                config,
                HaAction::ToggleLight { on: next }.into_request(config)?,
            )
            .await;
            snapshot.light_on = next;
            return Ok(confirm_action_snapshot_with_delays(
                config,
                &original,
                result.err().map(|err| err.to_string()),
                move |current| current.light_on == next,
                initial_delay,
                retry_count,
                retry_interval,
                "环境氛围照明尚未进入预期开关状态，继续等待刷新。",
            )
            .await);
        }
        ActionKind::StartupOnline => {
            send_startup_online(config).await?;
            snapshot.ac_available = config.ac_entity_id().is_some();
            snapshot.light_available = config.light_entity_id().is_some();

            if snapshot.ac_available {
                snapshot.ac.is_on = true;
            } else {
                snapshot.ac.is_on = false;
            }

            if snapshot.light_available {
                snapshot.light_on = true;
            } else {
                snapshot.light_on = false;
            }
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
