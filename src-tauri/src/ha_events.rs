use crate::models::AppConfig;
use serde_json::Value;
use std::time::Duration;

pub fn websocket_url_from_http_url(http_url: &str) -> String {
    let base = http_url.trim_end_matches('/');
    let ws_base = base
        .strip_prefix("https://")
        .map(|rest| format!("wss://{rest}"))
        .or_else(|| base.strip_prefix("http://").map(|rest| format!("ws://{rest}")))
        .unwrap_or_else(|| base.to_string());

    format!("{ws_base}/api/websocket")
}

pub fn entity_id_from_state_changed_event(message: &Value) -> Option<&str> {
    let message_type = message.get("type")?.as_str()?;
    if message_type != "event" {
        return None;
    }

    let event = message.get("event")?;
    if event.get("event_type")?.as_str()? != "state_changed" {
        return None;
    }

    event
        .get("data")?
        .get("entity_id")?
        .as_str()
}

pub fn should_refresh_snapshot(config: &AppConfig, entity_id: &str) -> bool {
    config.ac_entity_id() == Some(entity_id)
        || config.ambient_light_entity_id() == Some(entity_id)
        || config.main_light_entity_id() == Some(entity_id)
        || config.door_sign_light_entity_id() == Some(entity_id)
        || config.pc_entity_id() == Some(entity_id)
}

pub(crate) fn websocket_idle_ping_after() -> Duration {
    Duration::from_secs(30)
}

pub(crate) fn websocket_pong_timeout() -> Duration {
    Duration::from_secs(10)
}

pub(crate) fn should_send_idle_ping(last_seen: std::time::Instant, now: std::time::Instant) -> bool {
    now.duration_since(last_seen) >= websocket_idle_ping_after()
}

pub(crate) fn should_timeout_pending_ping(
    ping_sent_at: std::time::Instant,
    now: std::time::Instant,
) -> bool {
    now.duration_since(ping_sent_at) >= websocket_pong_timeout()
}

#[cfg(windows)]
mod windows_listener {
    use super::{
        entity_id_from_state_changed_event, should_refresh_snapshot, should_send_idle_ping,
        should_timeout_pending_ping, websocket_idle_ping_after, websocket_pong_timeout,
        websocket_url_from_http_url,
    };
    use crate::{log_line, refresh_snapshot_with_retry, models::AppConfig, SharedState};
    use futures_util::{SinkExt, StreamExt};
    use serde_json::{json, Value};
    use std::sync::OnceLock;
    use std::time::{Duration, Instant};
    use tauri::{AppHandle, Manager};
    use tokio::time::{sleep, Instant as TokioInstant};
    use tokio_tungstenite::{connect_async, tungstenite::Message};

    static LISTENER_STARTED: OnceLock<()> = OnceLock::new();

    pub(crate) fn spawn_state_listener_once(app: AppHandle, config: AppConfig) {
        if LISTENER_STARTED.set(()).is_err() {
            return;
        }

        tauri::async_runtime::spawn(async move {
            run_listener_forever(app, config).await;
        });
    }

    async fn run_listener_forever(app: AppHandle, config: AppConfig) {
        loop {
            if let Err(err) = connect_and_listen(&app, &config).await {
                log_line("WARN", format!("ha websocket listener stopped: {err}"));
            }

            sleep(Duration::from_secs(5)).await;
        }
    }

    async fn connect_and_listen(app: &AppHandle, config: &AppConfig) -> anyhow::Result<()> {
        let ws_url = websocket_url_from_http_url(&config.ha_url);
        let (mut socket, _) = connect_async(ws_url).await?;

        let auth_required = socket
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("home assistant websocket closed before auth"))??;
        let auth_required = auth_required.into_text()?;
        let auth_required: Value = serde_json::from_str(&auth_required)?;
        if auth_required.get("type").and_then(Value::as_str) != Some("auth_required") {
            return Err(anyhow::anyhow!("home assistant websocket did not request auth"));
        }

        socket
            .send(Message::Text(
                json!({
                    "type": "auth",
                    "access_token": config.token,
                })
                .to_string()
                .into(),
            ))
            .await?;

        let auth_ok = socket
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("home assistant websocket closed during auth"))??;
        let auth_ok = auth_ok.into_text()?;
        let auth_ok: Value = serde_json::from_str(&auth_ok)?;
        if auth_ok.get("type").and_then(Value::as_str) != Some("auth_ok") {
            return Err(anyhow::anyhow!("home assistant websocket auth failed"));
        }

        socket
            .send(Message::Text(
                json!({
                    "id": 1,
                    "type": "subscribe_events",
                    "event_type": "state_changed",
                })
                .to_string()
                .into(),
            ))
            .await?;

        log_line("INFO", "ha websocket listener connected");
        refresh_and_emit(app, config).await?;

        let (mut sink, mut stream) = socket.split();
        let mut last_seen = Instant::now();
        let mut ping_sent_at: Option<Instant> = None;
        let mut ping_nonce: u64 = 1;

        loop {
            let now = Instant::now();
            let deadline = if let Some(sent_at) = ping_sent_at {
                if should_timeout_pending_ping(sent_at, now) {
                    return Err(anyhow::anyhow!("home assistant websocket ping timeout"));
                }
                sent_at + websocket_pong_timeout()
            } else {
                last_seen + websocket_idle_ping_after()
            };

            let sleep_until = TokioInstant::from_std(deadline);
            let timer = tokio::time::sleep_until(sleep_until);
            tokio::pin!(timer);

            tokio::select! {
                maybe_message = stream.next() => {
                    let message = match maybe_message {
                        Some(message) => message?,
                        None => return Ok(()),
                    };

                    last_seen = Instant::now();
                    ping_sent_at = None;

                    match message {
                        Message::Text(text) => {
                            let payload: Value = serde_json::from_str(&text)?;
                            if let Some(entity_id) = entity_id_from_state_changed_event(&payload) {
                                if should_refresh_snapshot(config, entity_id) {
                                    refresh_and_emit(app, config).await?;
                                }
                            }
                        }
                        Message::Ping(payload) => {
                            sink.send(Message::Pong(payload)).await?;
                        }
                        Message::Pong(_) => {}
                        Message::Close(_) => return Ok(()),
                        _ => {}
                    }
                }
                _ = &mut timer => {
                    let now = Instant::now();

                    if let Some(sent_at) = ping_sent_at {
                        if should_timeout_pending_ping(sent_at, now) {
                            return Err(anyhow::anyhow!("home assistant websocket ping timeout"));
                        }
                        continue;
                    }

                    if should_send_idle_ping(last_seen, now) {
                        sink.send(Message::Ping(ping_nonce.to_be_bytes().to_vec().into())).await?;
                        ping_nonce = ping_nonce.wrapping_add(1);
                        ping_sent_at = Some(now);
                    }
                }
            }
        }
    }

    async fn refresh_and_emit(app: &AppHandle, config: &AppConfig) -> anyhow::Result<()> {
        let snapshot = refresh_snapshot_with_retry(config).await?;
        let shared = app.state::<SharedState>();
        if let Ok(mut state) = shared.0.lock() {
            *state = snapshot.clone();
        }
        let _ = app.emit_all("state-refresh", snapshot);
        Ok(())
    }

}

#[cfg(windows)]
pub(crate) use windows_listener::spawn_state_listener_once;

#[cfg(test)]
mod tests {
    use super::{
        entity_id_from_state_changed_event, should_refresh_snapshot,
        should_send_idle_ping, should_timeout_pending_ping, websocket_idle_ping_after,
        websocket_pong_timeout, websocket_url_from_http_url,
    };
    use crate::models::{AppConfig, DeviceIds};
    use std::time::{Duration, Instant};

    fn sample_config() -> AppConfig {
        AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(DeviceIds {
                ac: Some("climate.office_ac".into()),
                ambient_light: Some("switch.office_light".into()),
                main_light: Some("light.ceiling".into()),
                door_sign_light: Some("switch.door_sign".into()),
                ..Default::default()
            }),
        }
    }

    #[test]
    fn converts_http_url_to_websocket_url() {
        assert_eq!(
            websocket_url_from_http_url("https://ha.example.local"),
            "wss://ha.example.local/api/websocket"
        );
        assert_eq!(
            websocket_url_from_http_url("http://192.168.1.2:8123/"),
            "ws://192.168.1.2:8123/api/websocket"
        );
    }

    #[test]
    fn extracts_entity_id_from_state_changed_event() {
        let message = serde_json::json!({
            "type": "event",
            "event": {
                "event_type": "state_changed",
                "data": {
                    "entity_id": "climate.office_ac"
                }
            }
        });

        assert_eq!(
            entity_id_from_state_changed_event(&message),
            Some("climate.office_ac")
        );
    }

    #[test]
    fn accepts_only_bound_entities_for_refresh() {
        let config = sample_config();

        assert!(should_refresh_snapshot(&config, "climate.office_ac"));
        assert!(should_refresh_snapshot(&config, "switch.office_light"));
        assert!(should_refresh_snapshot(&config, "light.ceiling"));
        assert!(should_refresh_snapshot(&config, "switch.door_sign"));
        assert!(should_refresh_snapshot(&config, "input_boolean.pc_05_online"));
        assert!(!should_refresh_snapshot(&config, "climate.room2_ac"));
    }

    #[test]
    fn idle_ping_starts_after_threshold() {
        let now = Instant::now();

        assert!(!should_send_idle_ping(now - Duration::from_secs(29), now));
        assert!(should_send_idle_ping(now - Duration::from_secs(30), now));
        assert_eq!(websocket_idle_ping_after(), Duration::from_secs(30));
    }

    #[test]
    fn pending_ping_times_out_after_threshold() {
        let now = Instant::now();

        assert!(!should_timeout_pending_ping(now - Duration::from_secs(9), now));
        assert!(should_timeout_pending_ping(now - Duration::from_secs(10), now));
        assert_eq!(websocket_pong_timeout(), Duration::from_secs(10));
    }
}
