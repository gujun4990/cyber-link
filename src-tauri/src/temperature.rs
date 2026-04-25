use crate::models::ACState;
use serde_json::{json, Map, Value};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemperatureUnit {
    Celsius,
    Fahrenheit,
}

pub fn parse_double(attributes: &Value, keys: &[&str]) -> Option<f64> {
    for key in keys {
        if let Some(value) = attributes.get(*key) {
            if let Some(number) = value.as_f64() {
                return Some(number);
            }
            if let Some(number) = value.as_str().and_then(|text| text.parse::<f64>().ok()) {
                return Some(number);
            }
        }
    }

    None
}

pub fn parse_temperature_unit(attributes: &Value) -> Option<TemperatureUnit> {
    if let Some(unit) = attributes
        .get("temperature_unit")
        .or_else(|| attributes.get("unit_of_measurement"))
        .and_then(Value::as_str)
    {
        let normalized = unit.trim().to_ascii_lowercase();
        if normalized.contains('f') {
            return Some(TemperatureUnit::Fahrenheit);
        }
        if normalized.contains('c') {
            return Some(TemperatureUnit::Celsius);
        }
    }

    let readings = [
        parse_double(attributes, &["temperature"]),
        parse_double(attributes, &["target_temperature"]),
        parse_double(attributes, &["current_temperature"]),
        parse_double(attributes, &["min_temp"]),
        parse_double(attributes, &["max_temp"]),
    ];

    if readings.into_iter().flatten().any(|value| value > 45.0) {
        Some(TemperatureUnit::Fahrenheit)
    } else {
        Some(TemperatureUnit::Celsius)
    }
}

pub fn temperature_from_attributes(attributes: &Value) -> Option<i32> {
    let temperature = parse_double(
        attributes,
        &["temperature", "target_temperature", "current_temperature"],
    )?;

    let value = match parse_temperature_unit(attributes) {
        Some(TemperatureUnit::Fahrenheit) => fahrenheit_to_celsius(temperature),
        Some(TemperatureUnit::Celsius) | None => temperature,
    };

    Some(round_one_decimal(value).round() as i32)
}

pub fn normalize_temperature_for_entity(attributes: &Value, requested_celsius: i32) -> f64 {
    let unit = parse_temperature_unit(attributes);
    let min_temp = parse_double(attributes, &["min_temp"]);
    let max_temp = parse_double(attributes, &["max_temp"]);
    let step = parse_double(
        attributes,
        &["step", "temperature_step", "target_temp_step"],
    )
    .filter(|value| *value > 0.0);
    let anchor = min_temp.unwrap_or(0.0);

    let mut value = match unit {
        Some(TemperatureUnit::Fahrenheit) => celsius_to_fahrenheit(requested_celsius as f64),
        Some(TemperatureUnit::Celsius) | None => requested_celsius as f64,
    };

    if let Some(step) = step {
        value = (((value - anchor) / step).round() * step) + anchor;
    }

    if let Some(min_temp) = min_temp {
        value = value.max(min_temp);
    }
    if let Some(max_temp) = max_temp {
        value = value.min(max_temp);
    }

    value
}

pub fn normalize_temperature_for_celsius(attributes: &Value, requested_celsius: i32) -> i32 {
    let normalized_entity = normalize_temperature_for_entity(attributes, requested_celsius);
    let celsius = match parse_temperature_unit(attributes) {
        Some(TemperatureUnit::Fahrenheit) => fahrenheit_to_celsius(normalized_entity),
        Some(TemperatureUnit::Celsius) | None => normalized_entity,
    };

    round_one_decimal(celsius).round() as i32
}

pub fn normalize_temperature_for_ac_state(state: &ACState, requested_celsius: i32) -> (i32, i32) {
    let attributes = ac_state_temperature_attributes(state);
    let normalized = normalize_temperature_for_entity(&attributes, requested_celsius).round() as i32;
    let confirmed = normalize_temperature_for_celsius(&attributes, requested_celsius);

    (normalized, confirmed)
}

fn ac_state_temperature_attributes(state: &ACState) -> Value {
    let mut attributes = Map::new();

    if let Some(value) = state.min_temp {
        attributes.insert("min_temp".into(), json!(value));
    }
    if let Some(value) = state.max_temp {
        attributes.insert("max_temp".into(), json!(value));
    }
    if let Some(value) = state.target_temp_step {
        attributes.insert("target_temp_step".into(), json!(value));
    }
    if let Some(value) = state.temperature_unit.as_ref() {
        attributes.insert("temperature_unit".into(), json!(value));
    }
    if let Some(value) = state.unit_of_measurement.as_ref() {
        attributes.insert("unit_of_measurement".into(), json!(value));
    }

    Value::Object(attributes)
}

fn celsius_to_fahrenheit(value: f64) -> f64 {
    (value * 9.0 / 5.0) + 32.0
}

fn fahrenheit_to_celsius(value: f64) -> f64 {
    (value - 32.0) * 5.0 / 9.0
}

fn round_one_decimal(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

#[cfg(test)]
mod tests {
    use super::{
        normalize_temperature_for_celsius, normalize_temperature_for_entity,
        parse_temperature_unit, temperature_from_attributes, TemperatureUnit,
    };

    #[test]
    fn parses_temperature_unit_from_explicit_attribute() {
        let attrs = serde_json::json!({"temperature_unit": "°F"});
        assert_eq!(
            parse_temperature_unit(&attrs),
            Some(TemperatureUnit::Fahrenheit)
        );
    }

    #[test]
    fn parses_temperature_unit_from_celsius_hint() {
        let attrs = serde_json::json!({"unit_of_measurement": "°C"});
        assert_eq!(
            parse_temperature_unit(&attrs),
            Some(TemperatureUnit::Celsius)
        );
    }

    #[test]
    fn converts_temperature_readings_to_celsius() {
        let attrs = serde_json::json!({"temperature": 77, "temperature_unit": "°F"});
        assert_eq!(temperature_from_attributes(&attrs), Some(25));
    }

    #[test]
    fn normalizes_requested_temperature_for_entity_unit() {
        let attrs = serde_json::json!({
            "temperature_unit": "°F",
            "min_temp": 60,
            "max_temp": 80,
            "target_temp_step": 1,
        });

        assert_eq!(normalize_temperature_for_entity(&attrs, 26), 79.0);
    }

    #[test]
    fn normalizes_requested_temperature_back_to_celsius() {
        let attrs = serde_json::json!({
            "temperature_unit": "°F",
            "min_temp": 60,
            "max_temp": 80,
            "target_temp_step": 1,
        });

        assert_eq!(normalize_temperature_for_celsius(&attrs, 26), 26);
    }
}
