use crate::models::{AppConfig, HaEntityState, HaRequest};
use crate::temperature::{normalize_temperature_for_celsius, normalize_temperature_for_entity};
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
            .light_entity_id()
            .ok_or_else(|| anyhow!("light entity is not configured"))?
    };

    Ok(entity_id)
}

fn climate_request(config: &AppConfig, service: &str) -> Result<HaRequest> {
    let entity_id = entity_id(config, true)?;
    Ok(HaRequest {
        url: format!("{}/api/services/climate/{}", base_url(config), service),
        body: request_body(entity_id),
    })
}

fn light_request(config: &AppConfig, service: &str) -> Result<HaRequest> {
    let entity_id = entity_id(config, false)?;
    Ok(HaRequest {
        url: format!("{}/api/services/light/{}", base_url(config), service),
        body: request_body(entity_id),
    })
}

pub fn climate_turn_on_request(config: &AppConfig) -> Result<HaRequest> {
    climate_request(config, "turn_on")
}

pub fn climate_turn_off_request(config: &AppConfig) -> Result<HaRequest> {
    climate_request(config, "turn_off")
}

pub fn light_turn_on_request(config: &AppConfig) -> Result<HaRequest> {
    light_request(config, "turn_on")
}

pub fn light_turn_off_request(config: &AppConfig) -> Result<HaRequest> {
    light_request(config, "turn_off")
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

pub async fn climate_temperature_targets(config: &AppConfig, requested: i32) -> Result<(i32, i32)> {
    let entity_id = entity_id(config, true)?;
    let current_state = fetch_ha_entity_state(config, entity_id).await?;

    let request_temperature =
        normalize_temperature_for_entity(&current_state.attributes, requested).round() as i32;
    let confirm_temperature =
        normalize_temperature_for_celsius(&current_state.attributes, requested);

    Ok((request_temperature, confirm_temperature))
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
    send_ha_request_with_timeout(config, request, Duration::from_secs(2)).await
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
