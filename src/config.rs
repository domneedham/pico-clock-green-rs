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

/// Time preference representation.
#[derive(Copy, Clone)]
pub enum TimePreference {
    /// 12hr.
    Twelve,

    /// 24hr.
    TwentyFour,
}

/// Manage active configuration.
pub struct Config {
    /// Whether the clock should beep on the hour.
    hourly_ring: bool,

    /// The users temperature reporting preference.
    temp_pref: TemperaturePreference,

    /// Whether the clock should auto scroll and show the temperature.
    auto_scroll_temp: bool,

    /// The users time representation preference.
    time_pref: TimePreference,
}

impl Config {
    /// Create a new default config.
    pub const fn new() -> Self {
        Self {
            hourly_ring: false,
            temp_pref: TemperaturePreference::Celcius,
            auto_scroll_temp: true,
            time_pref: TimePreference::TwentyFour,
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

    /// Toggle the users temperature preference.
    pub fn toggle_temperature_preference(&mut self) {
        match self.get_temperature_preference() {
            TemperaturePreference::Celcius => {
                self.set_temperature_preference(TemperaturePreference::Fahrenheit)
            }
            TemperaturePreference::Fahrenheit => {
                self.set_temperature_preference(TemperaturePreference::Celcius)
            }
        }
    }

    /// Get the auto scroll temperature state.
    pub fn get_auto_scroll_temp(&self) -> bool {
        self.auto_scroll_temp
    }

    /// Set the auto scroll temperature state.
    pub fn set_auto_scroll_temp(&mut self, new_state: bool) {
        self.auto_scroll_temp = new_state;
    }

    /// Get the users temperature preference.
    pub fn get_time_preference(&self) -> TimePreference {
        self.time_pref
    }

    /// Set the users time preference.
    pub fn set_time_preference(&mut self, new_state: TimePreference) {
        self.time_pref = new_state;
    }

    /// Toggle the users time preference.
    pub fn toggle_time_preference(&mut self) {
        match self.get_time_preference() {
            TimePreference::Twelve => self.set_time_preference(TimePreference::TwentyFour),
            TimePreference::TwentyFour => self.set_time_preference(TimePreference::Twelve),
        }
    }
}

/// Static reference to the config so it can be accessed by all otehr apps.
pub static CONFIG: Mutex<ThreadModeRawMutex, RefCell<Config>> =
    Mutex::new(RefCell::new(Config::new()));
