use core::cell::RefCell;

use defmt::info;
use embassy_rp::flash::{Async, Flash, ERASE_SIZE};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};

pub const FLASH_SIZE: usize = 2 * 1024 * 1024;
const ADDR_OFFSET: u32 = 0x100000;

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

/// Time colon preference representation.
#[derive(Copy, Clone, PartialEq)]
pub enum TimeColonPreference {
    /// Do not blink the colon.
    Solid,

    /// Blink the colon.
    Blink,

    /// Show the alternate blinking colon.
    Alt,
}

/// Manage active configuration.
pub struct Config {
    /// Whether the clock should beep on the hour.
    hourly_ring: bool,

    /// The users colon blinking preference.
    time_colon_pref: TimeColonPreference,

    /// The users temperature reporting preference.
    temp_pref: TemperaturePreference,

    /// Whether the clock should auto scroll and show the temperature.
    auto_scroll_temp: bool,

    /// The users time representation preference.
    time_pref: TimePreference,

    /// Whether the display should use auto brightness or not.
    autolight: bool,
}

impl Config {
    /// Init the config.
    pub async fn new(
        mut flash: Flash<'static, embassy_rp::peripherals::FLASH, Async, FLASH_SIZE>,
    ) -> Self {
        let mut read_buf = [0u8; ERASE_SIZE];
        flash.write(ADDR_OFFSET, "Hello world".as_bytes()).unwrap();
        flash.read(ADDR_OFFSET, &mut read_buf).unwrap();
        info!("Contents start with {=[u8]}", read_buf);

        Self {
            hourly_ring: false,
            time_colon_pref: TimeColonPreference::Blink,
            temp_pref: TemperaturePreference::Celcius,
            auto_scroll_temp: true,
            time_pref: TimePreference::TwentyFour,
            autolight: true,
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

    /// Get the time colon preference.
    pub fn get_time_colon_preference(&self) -> TimeColonPreference {
        self.time_colon_pref
    }

    /// Set the users time colon preference.
    pub fn set_time_colon_preference(&mut self, new_state: TimeColonPreference) {
        self.time_colon_pref = new_state;
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

    /// Get the autolight state.
    pub fn get_autolight(&self) -> bool {
        self.autolight
    }

    /// Set the autolight state.
    pub fn set_autolight(&mut self, new_state: bool) {
        self.autolight = new_state;
    }

    /// Toggle the autolight value. Return the new value.
    pub fn toggle_autolight(&mut self) -> bool {
        let state = !self.autolight;
        self.set_autolight(state);
        state
    }
}

/// Static reference to the config so it can be accessed by all otehr apps.
pub static CONFIG: Mutex<ThreadModeRawMutex, RefCell<Option<Config>>> =
    Mutex::new(RefCell::new(None));

pub async fn init(flash: Flash<'static, embassy_rp::peripherals::FLASH, Async, FLASH_SIZE>) {
    let config = Config::new(flash).await;
    CONFIG.lock().await.replace(Some(config));
}
