use core::cell::RefCell;

use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};

/// Temperature preference representation.
#[derive(Copy, Clone)]
pub enum TemperaturePreference {
    /// Celcius.
    Celcius,

    /// Fahrenheit.
    Fahrenheit,
}

/// Manage active configuration.
pub struct Config {
    /// Whether the clock should beep on the hour.
    hourly_ring: bool,

    /// The users temperature reporting preference.
    temp_pref: TemperaturePreference,
}

impl Config {
    /// Create a new default config.
    pub const fn new() -> Self {
        Self {
            hourly_ring: false,
            temp_pref: TemperaturePreference::Celcius,
        }
    }

    /// Get the hourly ring state.
    pub fn get_hourly_ring(&self) -> bool {
        self.hourly_ring
    }

    /// Set the hourly ring state.
    pub fn set_hourly_ring(&mut self, new_state: bool) {
        self.hourly_ring = new_state;
    }

    /// Get the users temperature preference.
    pub fn get_temperature_preference(&self) -> TemperaturePreference {
        self.temp_pref
    }

    /// Set the users temperature preference.
    pub fn set_temperature_preference(&mut self, new_state: TemperaturePreference) {
        self.temp_pref = new_state;
    }
}

/// Static reference to the config so it can be accessed by all otehr apps.
pub static CONFIG: Mutex<ThreadModeRawMutex, RefCell<Config>> =
    Mutex::new(RefCell::new(Config::new()));
