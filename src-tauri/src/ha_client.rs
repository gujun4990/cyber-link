use crate::models::{AppConfig, HaEntityState, HaRequest};
use crate::temperature::normalize_temperature_for_entity;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde_json::{json, Value};
use std::sync::OnceLock;
use std::time::Duration;

static HA_CLIENT: OnceLock<Client> = OnceLock::new();

fn ha_client() -> &'static Client {
    HA_CLIENT.get_or_init(|| {
        Client::builder()
            .pool_idle_timeout(Some(Duration::from_secs(30)))
            .pool_max_idle_per_host(2)
            .build()
            .expect("failed to build Home Assistant client")
    })
}

fn base_url(config: &AppConfig) -> &str {
    config.ha_url.trim_end_matches('/')
}

fn request_body(entity_id: &str) -> Value {
    json!({"entity_id": entity_id})
}

fn entity_id<'a>(config: &'a AppConfig, is_ac: bool) -> Result<&'a str> {
    let entity_id = if is_ac {
        config
            .ac_entity_id()
            .ok_or_else(|| anyhow!("AC entity is not configured"))?
    } else {
        config
            .ambient_light_entity_id()
            .ok_or_else(|| anyhow!("ambient light entity is not configured"))?
    };

    Ok(entity_id)
}

fn entity_domain(entity_id: &str) -> Result<&str> {
    entity_id
        .split_once('.')
        .map(|(domain, _)| domain)
        .ok_or_else(|| anyhow!("entity_id must contain a domain prefix"))
}

fn generic_request(config: &AppConfig, entity_id: &str, service: &str) -> Result<HaRequest> {
    let domain = entity_domain(entity_id)?;

    Ok(HaRequest {
        url: format!("{}/api/services/{}/{}", base_url(config), domain, service),
        body: request_body(entity_id),
    })
}

pub(crate) fn entity_turn_on_request(config: &AppConfig, entity_id: &str) -> Result<HaRequest> {
    generic_request(config, entity_id, "turn_on")
}

pub(crate) fn entity_turn_off_request(config: &AppConfig, entity_id: &str) -> Result<HaRequest> {
    generic_request(config, entity_id, "turn_off")
}

fn climate_request(config: &AppConfig, service: &str) -> Result<HaRequest> {
    let entity_id = entity_id(config, true)?;
    generic_request(config, entity_id, service)
}

fn switch_request(config: &AppConfig, service: &str) -> Result<HaRequest> {
    let entity_id = entity_id(config, false)?;
    generic_request(config, entity_id, service)
}

pub fn climate_turn_on_request(config: &AppConfig) -> Result<HaRequest> {
    climate_request(config, "turn_on")
}

pub fn climate_turn_off_request(config: &AppConfig) -> Result<HaRequest> {
    climate_request(config, "turn_off")
}

pub fn switch_turn_on_request(config: &AppConfig) -> Result<HaRequest> {
    switch_request(config, "turn_on")
}

pub fn switch_turn_off_request(config: &AppConfig) -> Result<HaRequest> {
    switch_request(config, "turn_off")
}

pub async fn normalized_climate_temperature(config: &AppConfig, requested: i32) -> Result<i32> {
    let entity_id = entity_id(config, true)?;
    let current_state = fetch_ha_entity_state(config, entity_id).await?;

    Ok(normalize_temperature_for_entity(&current_state.attributes, requested).round() as i32)
}

pub fn climate_set_temperature_request(config: &AppConfig, temperature: i32) -> Result<HaRequest> {
    let entity_id = entity_id(config, true)?;

    Ok(HaRequest {
        url: format!("{}/api/services/climate/set_temperature", base_url(config)),
        body: json!({
            "entity_id": entity_id,
            "temperature": temperature_json_value(temperature as f64),
        }),
    })
}

pub fn normalize_climate_temperature(state: &HaEntityState, requested: i32) -> f64 {
    normalize_temperature_for_entity(&state.attributes, requested)
}

pub async fn fetch_ha_entity_state(config: &AppConfig, entity_id: &str) -> Result<HaEntityState> {
    Ok(ha_client()
        .get(format!("{}/api/states/{}", base_url(config), entity_id))
        .bearer_auth(&config.token)
        .send()
        .await?
        .error_for_status()?
        .json::<HaEntityState>()
        .await?)
}

pub async fn send_ha_request(config: &AppConfig, request: HaRequest) -> Result<()> {
    send_ha_request_with_timeout(config, request, Duration::from_secs(10)).await
}

pub async fn send_ha_request_with_timeout(
    config: &AppConfig,
    request: HaRequest,
    timeout: Duration,
) -> Result<()> {
    ha_client()
        .post(request.url)
        .bearer_auth(&config.token)
        .json(&request.body)
        .timeout(timeout)
        .send()
        .await?
        .error_for_status()?;
    Ok(())
}

fn temperature_json_value(temperature: f64) -> Value {
    if (temperature.fract()).abs() < f64::EPSILON {
        json!(temperature as i64)
    } else {
        json!(temperature)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    fn sample_config() -> AppConfig {
        AppConfig {
            ha_url: "https://ha.example.local".into(),
            token: "secret".into(),
            pc_entity_id: Some("input_boolean.pc_05_online".into()),
            entity_id: Some(crate::models::DeviceIds {
                ac: Some("climate.office_ac".into()),
                ambient_light: Some("switch.office_light".into()),
                main_light: Some("light.ceiling".into()),
                door_sign_light: Some("switch.door_sign".into()),
            }),
        }
    }

    #[test]
    fn generic_request_uses_entity_prefix_as_domain() {
        let request = generic_request(&sample_config(), "switch.office_light", "turn_on")
            .expect("request");

        assert_eq!(request.url, "https://ha.example.local/api/services/switch/turn_on");
        assert_eq!(request.body, serde_json::json!({"entity_id": "switch.office_light"}));
    }

    #[tokio::test]
    async fn send_ha_request_waits_long_enough_for_slow_home_assistant() {
        std::env::set_var("NO_PROXY", "127.0.0.1,localhost");
        std::env::set_var("no_proxy", "127.0.0.1,localhost");
        std::env::set_var("HTTP_PROXY", "");
        std::env::set_var("HTTPS_PROXY", "");
        std::env::set_var("http_proxy", "");
        std::env::set_var("https_proxy", "");

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .expect("listener");
        let addr = listener.local_addr().expect("listener addr");

        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.expect("accept request");
            let mut buf = vec![0u8; 4096];
            let _ = socket.read(&mut buf).await.expect("read request");
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            let body = r#"{"state":"ok","attributes":{}}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket
                .write_all(response.as_bytes())
                .await
                .expect("write response");
        });

        let request = HaRequest {
            url: format!("http://{addr}/api/services/climate/turn_on"),
            body: serde_json::json!({"entity_id": "climate.office_ac"}),
        };

        let result = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            send_ha_request(&sample_config(), request),
        )
        .await
        .expect("request should not time out");

        server.await.expect("server task");

        assert!(result.is_ok());
    }
}
