use core::cell::RefCell;

use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};

/// Manage active configuration.
pub struct Config {
    /// Whether the clock should beep on the hour.
    hourly_ring: bool,
}

impl Config {
    /// Create a new default config.
    pub const fn new() -> Self {
        Self { hourly_ring: false }
    }

    /// Get the hourly ring state.
    pub fn get_hourly_ring(&self) -> bool {
        self.hourly_ring
    }

    /// Set the hourly ring state.
    pub fn set_hourly_ring(&mut self, new_state: bool) {
        self.hourly_ring = new_state;
    }
}

/// Static reference to the config so it can be accessed by all otehr apps.
pub static CONFIG: Mutex<ThreadModeRawMutex, RefCell<Config>> =
    Mutex::new(RefCell::new(Config::new()));
