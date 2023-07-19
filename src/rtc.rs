use core::cell::RefCell;
use ds323x::Ds323x;
use embassy_rp::{i2c, peripherals::I2C1};
use embassy_sync::blocking_mutex::raw::ThreadModeRawMutex;

pub struct Ds3231<'a>(
    pub  Ds323x<
        ds323x::interface::I2cInterface<i2c::I2c<'a, I2C1, i2c::Blocking>>,
        ds323x::ic::DS3231,
    >,
);

pub static RTC: embassy_sync::mutex::Mutex<ThreadModeRawMutex, RefCell<Option<Ds3231>>> =
    embassy_sync::mutex::Mutex::new(RefCell::new(None));
