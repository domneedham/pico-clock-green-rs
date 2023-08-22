use crate::{
    config::{self, TemperaturePreference},
    rtc,
};

/// Get the temperature preference.
pub async fn get_temperature_preference() -> TemperaturePreference {
    config::get_temperature_preference().await
}

/// Get the temperature based on the current user preference.
pub async fn get_temperature_off_preference() -> f32 {
    let pref = get_temperature_preference().await;
    match pref {
        config::TemperaturePreference::Celcius => get_celcius().await,
        config::TemperaturePreference::Fahrenheit => get_fahrenheit().await,
    }
}

/// Get the temperature in celcius.
pub async fn get_celcius() -> f32 {
    rtc::temperature::get_temperature().await
}

/// Get the temperature in fahrenheit.
pub async fn get_fahrenheit() -> f32 {
    let temp = rtc::temperature::get_temperature().await;
    (temp * 1.8) + 32.0
}
