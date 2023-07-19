use chrono::NaiveDateTime;
use core::cell::RefCell;
use ds323x::{DateTimeAccess, Ds323x};
use embassy_rp::{i2c, peripherals::I2C1};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};

pub struct Ds3231<'a>(
    pub  Ds323x<
        ds323x::interface::I2cInterface<i2c::I2c<'a, I2C1, i2c::Blocking>>,
        ds323x::ic::DS3231,
    >,
);

static RTC: Mutex<ThreadModeRawMutex, RefCell<Option<Ds3231>>> = Mutex::new(RefCell::new(None));

pub async fn init(ds3231: Ds3231<'static>) {
    RTC.lock().await.replace(Some(ds3231));
}

pub async fn get_datetime() -> NaiveDateTime {
    RTC.lock()
        .await
        .borrow_mut()
        .as_mut()
        .unwrap()
        .0
        .datetime()
        .unwrap()
}
