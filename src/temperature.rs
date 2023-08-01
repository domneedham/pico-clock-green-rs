use crate::{config, rtc};

pub async fn get_temperature_off_preference() -> f32 {
    match config::CONFIG
        .lock()
        .await
        .borrow()
        .get_temperature_preference()
    {
        config::TemperaturePreference::Celcius => get_celcius().await,
        config::TemperaturePreference::Fahrenheit => get_fahrenheit().await,
    }
}

/// Get the temperature in celcius.
pub async fn get_celcius() -> f32 {
    let temp = rtc::temperature::get_temperature().await;
    temp
}

/// Get the temperature in fahrenheit.
pub async fn get_fahrenheit() -> f32 {
    let temp = rtc::temperature::get_temperature().await;
    (temp * 1.8) + 32.0
}
