use core::cell::RefCell;

use embassy_rp::flash::{Async, Flash, ERASE_SIZE};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};

use self::flash_config::FlashOveride;

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

/// All the configuration options that can be edited at runtime.
pub struct ConfigOptions {
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

/// Manage active configuration.
pub struct Config {
    /// The flash memory peripheral.
    flash: Flash<'static, embassy_rp::peripherals::FLASH, Async, { flash_config::FLASH_SIZE }>,

    /// The config options.
    config_options: ConfigOptions,
}

impl Config {
    /// Init the config.
    pub async fn new(
        mut flash: Flash<
            'static,
            embassy_rp::peripherals::FLASH,
            Async,
            { flash_config::FLASH_SIZE },
        >,
    ) -> Self {
        let bytes = flash.read_all();

        let hourly_ring = flash_config::hourly_ring_from_bytes(&bytes);
        let time_colon_pref = flash_config::time_colon_from_bytes(&bytes);
        let temp_pref = flash_config::temp_pref_from_bytes(&bytes);
        let auto_scroll_temp = flash_config::auto_scroll_temp_from_bytes(&bytes);
        let time_pref = flash_config::time_pref_from_bytes(&bytes);
        let autolight = flash_config::autolight_from_bytes(&bytes);

        Self {
            flash,
            config_options: ConfigOptions {
                hourly_ring,
                time_colon_pref,
                temp_pref,
                auto_scroll_temp,
                time_pref,
                autolight,
            },
        }
    }
}

impl Config {
    /// Set the hourly ring state.
    fn set_hourly_ring(&mut self, new_state: bool) {
        self.config_options.hourly_ring = new_state;
        self.flash.write_all(&self.config_options);
    }

    /// Set the users time colon preference.
    fn set_time_colon_preference(&mut self, new_state: TimeColonPreference) {
        self.config_options.time_colon_pref = new_state;
        self.flash.write_all(&self.config_options);
    }

    /// Set the users temperature preference.
    fn set_temperature_preference(&mut self, new_state: TemperaturePreference) {
        self.config_options.temp_pref = new_state;
        self.flash.write_all(&self.config_options);
    }

    /// Set the auto scroll temperature state.
    fn set_auto_scroll_temp(&mut self, new_state: bool) {
        self.config_options.auto_scroll_temp = new_state;
        self.flash.write_all(&self.config_options);
    }

    /// Set the users time preference.
    fn set_time_preference(&mut self, new_state: TimePreference) {
        self.config_options.time_pref = new_state;
        self.flash.write_all(&self.config_options);
    }

    /// Set the autolight state.
    fn set_autolight(&mut self, new_state: bool) {
        self.config_options.autolight = new_state;
        self.flash.write_all(&self.config_options);
    }
}

/// Static reference to the config so it can be accessed by all otehr apps.
static CONFIG: Mutex<ThreadModeRawMutex, RefCell<Option<Config>>> = Mutex::new(RefCell::new(None));

/// Get hourly ring state.
pub async fn get_hourly_ring() -> bool {
    let guard = CONFIG.lock().await;
    let state = guard.borrow().as_ref().unwrap().config_options.hourly_ring;
    drop(guard);
    state
}

/// Set the hourly ring state.
pub async fn set_hourly_ring(new_state: bool) {
    let guard = CONFIG.lock().await;

    guard
        .borrow_mut()
        .as_mut()
        .unwrap()
        .set_hourly_ring(new_state);

    drop(guard);
}

/// Get the time colon preference.
pub async fn get_time_colon_preference() -> TimeColonPreference {
    let guard = CONFIG.lock().await;
    let state = guard
        .borrow()
        .as_ref()
        .unwrap()
        .config_options
        .time_colon_pref;
    drop(guard);
    state
}

/// Set the time colon preference.
pub async fn set_time_colon_preference(new_state: TimeColonPreference) {
    let guard = CONFIG.lock().await;

    guard
        .borrow_mut()
        .as_mut()
        .unwrap()
        .set_time_colon_preference(new_state);

    drop(guard);
}

/// Get the temperature preference.
pub async fn get_temperature_preference() -> TemperaturePreference {
    let guard = CONFIG.lock().await;
    let state = guard.borrow().as_ref().unwrap().config_options.temp_pref;
    drop(guard);
    state
}

/// Toggle the temperature preference.
pub async fn toggle_temperature_preference() {
    let guard = CONFIG.lock().await;
    let state = guard.borrow().as_ref().unwrap().config_options.temp_pref;
    match state {
        TemperaturePreference::Celcius => guard
            .borrow_mut()
            .as_mut()
            .unwrap()
            .set_temperature_preference(TemperaturePreference::Fahrenheit),
        TemperaturePreference::Fahrenheit => guard
            .borrow_mut()
            .as_mut()
            .unwrap()
            .set_temperature_preference(TemperaturePreference::Celcius),
    }
    drop(guard);
}

/// Get the auto scroll preference.
pub async fn get_auto_scroll_temp() -> bool {
    let guard = CONFIG.lock().await;
    let state = guard
        .borrow()
        .as_ref()
        .unwrap()
        .config_options
        .auto_scroll_temp;
    drop(guard);
    state
}

/// Set the auto scroll preference.
pub async fn set_auto_scroll_temp(new_state: bool) {
    let guard = CONFIG.lock().await;

    guard
        .borrow_mut()
        .as_mut()
        .unwrap()
        .set_auto_scroll_temp(new_state);

    drop(guard);
}

/// Get the time preference.
pub async fn get_time_preference() -> TimePreference {
    let guard = CONFIG.lock().await;
    let state = guard.borrow().as_ref().unwrap().config_options.time_pref;
    drop(guard);
    state
}

/// Toggle the time preference.
pub async fn toggle_time_preference() {
    let guard = CONFIG.lock().await;
    let state = guard.borrow().as_ref().unwrap().config_options.time_pref;

    match state {
        TimePreference::Twelve => guard
            .borrow_mut()
            .as_mut()
            .unwrap()
            .set_time_preference(TimePreference::TwentyFour),
        TimePreference::TwentyFour => guard
            .borrow_mut()
            .as_mut()
            .unwrap()
            .set_time_preference(TimePreference::Twelve),
    }

    drop(guard);
}

/// Get the autolight preference.
pub async fn get_autolight() -> bool {
    let guard = CONFIG.lock().await;
    let state = guard.borrow().as_ref().unwrap().config_options.autolight;
    drop(guard);
    state
}

/// Toggle the autolight preference.
pub async fn toggle_autolight() -> bool {
    let guard = CONFIG.lock().await;

    let state = guard.borrow().as_ref().unwrap().config_options.autolight;
    guard.borrow_mut().as_mut().unwrap().set_autolight(!state);

    drop(guard);
    !state
}

/// Init the config. Must have an initialised flash memory.
pub async fn init(
    flash: Flash<'static, embassy_rp::peripherals::FLASH, Async, { flash_config::FLASH_SIZE }>,
) {
    let config = Config::new(flash).await;
    CONFIG.lock().await.replace(Some(config));
}

/// Flash memory read/write for config.
pub mod flash_config {
    use super::*;

    /// The flash size.
    pub const FLASH_SIZE: usize = 2 * 1024 * 1024;

    /// The initial offset of where to save the config in flash.
    pub const ADDR_OFFSET: u32 = 0x100000;

    /// The offset and end offset for hourly ring.
    const HOURLY_RING: (usize, usize) = (10, 11);
    /// The offset and end offset for time colon preference.
    const TIME_COLON_PREF: (usize, usize) = (HOURLY_RING.0 + 10, HOURLY_RING.0 + 11);
    /// The offset and end offset for temperature preference.
    const TEMP_PREF: (usize, usize) = (TIME_COLON_PREF.0 + 10, TIME_COLON_PREF.0 + 11);
    /// The offset and end offset for auto scrolling features.
    const AUTO_SCROLL_TEMP: (usize, usize) = (TEMP_PREF.0 + 10, TEMP_PREF.0 + 11);
    /// The offset and end offset for time hour preference.
    const TIME_PREF: (usize, usize) = (AUTO_SCROLL_TEMP.0 + 10, AUTO_SCROLL_TEMP.0 + 11);
    /// The offset and end offset for autolight.
    const AUTOLIGHT: (usize, usize) = (TIME_PREF.0 + 10, TIME_PREF.0 + 11);

    /// Bytes to use to reperesent a false value.
    const FALSE_BYTES: u8 = 0x00;

    /// Bytes to use to represent a true value.
    const TRUE_BYTES: u8 = 0x01;

    /// Trait to overload embassy flash.
    pub trait FlashOveride {
        /// Read all flash bytes from *ADDR_OFFSET*.
        fn read_all(&mut self) -> [u8; ERASE_SIZE];

        /// Write all config into flash.
        fn write_all(&mut self, state: &ConfigOptions);
    }

    impl FlashOveride for Flash<'static, embassy_rp::peripherals::FLASH, Async, FLASH_SIZE> {
        fn read_all(&mut self) -> [u8; ERASE_SIZE] {
            let mut read_buf = [0u8; ERASE_SIZE];
            self.blocking_read(ADDR_OFFSET, &mut read_buf).unwrap();
            read_buf
        }

        fn write_all(&mut self, state: &ConfigOptions) {
            // erase everything first
            self.blocking_erase(ADDR_OFFSET, ADDR_OFFSET + ERASE_SIZE as u32)
                .unwrap();

            let mut read_buf = [0u8; ERASE_SIZE];
            read_buf[HOURLY_RING.0] = hourly_ring_to_bytes(state.hourly_ring);
            read_buf[TIME_COLON_PREF.0] = time_colon_to_bytes(state.time_colon_pref);
            read_buf[TEMP_PREF.0] = temp_pref_to_bytes(state.temp_pref);
            read_buf[AUTO_SCROLL_TEMP.0] = auto_scroll_temp_to_bytes(state.auto_scroll_temp);
            read_buf[TIME_PREF.0] = time_pref_to_bytes(state.time_pref);
            read_buf[AUTOLIGHT.0] = autolight_to_bytes(state.autolight);

            self.blocking_write(ADDR_OFFSET, &read_buf).unwrap();
        }
    }

    /// Get the hourly ring config from the full flash byte array.
    pub fn hourly_ring_from_bytes(bytes: &[u8; ERASE_SIZE]) -> bool {
        let state_bytes = &bytes[HOURLY_RING.0..HOURLY_RING.1];
        if state_bytes == [TRUE_BYTES] {
            return true;
        }

        false
    }

    /// Convert the hourly ring state to bytes.
    pub fn hourly_ring_to_bytes(state: bool) -> u8 {
        if state {
            TRUE_BYTES
        } else {
            FALSE_BYTES
        }
    }

    /// Get the time colon preference config from the full flash byte array.
    pub fn time_colon_from_bytes(bytes: &[u8; ERASE_SIZE]) -> TimeColonPreference {
        let state_bytes = &bytes[TIME_COLON_PREF.0..TIME_COLON_PREF.1];
        match state_bytes {
            [0x00] => TimeColonPreference::Alt,
            [0x01] => TimeColonPreference::Blink,
            [0x02] => TimeColonPreference::Solid,
            _ => TimeColonPreference::Blink,
        }
    }

    /// Convert the time colon preference state to bytes.
    pub fn time_colon_to_bytes(state: TimeColonPreference) -> u8 {
        match state {
            TimeColonPreference::Alt => 0x00,
            TimeColonPreference::Blink => 0x01,
            TimeColonPreference::Solid => 0x02,
        }
    }

    /// Get the temperature preference config from the full flash byte array.
    pub fn temp_pref_from_bytes(bytes: &[u8; ERASE_SIZE]) -> TemperaturePreference {
        let state_bytes = &bytes[TEMP_PREF.0..TEMP_PREF.1];
        match state_bytes {
            [0x00] => TemperaturePreference::Celcius,
            [0x01] => TemperaturePreference::Fahrenheit,
            _ => TemperaturePreference::Celcius,
        }
    }

    /// Convert the temperature preference state to bytes.
    pub fn temp_pref_to_bytes(state: TemperaturePreference) -> u8 {
        match state {
            TemperaturePreference::Celcius => 0x00,
            TemperaturePreference::Fahrenheit => 0x01,
        }
    }

    /// Get the auto scroll feature config from the full flash byte array.
    pub fn auto_scroll_temp_from_bytes(bytes: &[u8; ERASE_SIZE]) -> bool {
        let state_bytes = &bytes[AUTO_SCROLL_TEMP.0..AUTO_SCROLL_TEMP.1];
        if state_bytes == [TRUE_BYTES] {
            return true;
        }

        false
    }

    /// Convert the auto scroll feature state to bytes.
    pub fn auto_scroll_temp_to_bytes(state: bool) -> u8 {
        if state {
            TRUE_BYTES
        } else {
            FALSE_BYTES
        }
    }

    /// Get the time preference config from the full flash byte array.
    pub fn time_pref_from_bytes(bytes: &[u8; ERASE_SIZE]) -> TimePreference {
        let state_bytes = &bytes[TIME_PREF.0..TIME_PREF.1];
        match state_bytes {
            [0x00] => TimePreference::Twelve,
            [0x01] => TimePreference::TwentyFour,
            _ => TimePreference::TwentyFour,
        }
    }

    /// Convert the time preference state to bytes.
    pub fn time_pref_to_bytes(state: TimePreference) -> u8 {
        match state {
            TimePreference::Twelve => 0x00,
            TimePreference::TwentyFour => 0x01,
        }
    }

    /// Get the autolight config from the full flash byte array.
    pub fn autolight_from_bytes(bytes: &[u8; ERASE_SIZE]) -> bool {
        let state_bytes = &bytes[AUTOLIGHT.0..AUTOLIGHT.1];
        if state_bytes == [TRUE_BYTES] {
            return true;
        }

        false
    }

    /// Convert the autolight state to bytes.
    pub fn autolight_to_bytes(state: bool) -> u8 {
        if state {
            TRUE_BYTES
        } else {
            FALSE_BYTES
        }
    }
}
