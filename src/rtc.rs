use chrono::{Datelike, NaiveDateTime, Timelike};
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

pub async fn get_hour() -> u32 {
    let datetime = get_datetime().await;
    datetime.hour()
}

pub async fn get_minute() -> u32 {
    let datetime = get_datetime().await;
    datetime.minute()
}

pub async fn get_day() -> u32 {
    let datetime = get_datetime().await;
    datetime.day()
}

pub async fn get_month() -> u32 {
    let datetime = get_datetime().await;
    datetime.month()
}

pub async fn set_hour(hour: u32) {
    let current_datetime = get_datetime().await;
    let new_datetime = current_datetime
        .with_hour(hour)
        .unwrap()
        .with_second(0)
        .unwrap();
    set_datetime(&new_datetime).await;
}

pub async fn set_minute(minute: u32) {
    let current_datetime = get_datetime().await;
    let new_datetime = current_datetime
        .with_minute(minute)
        .unwrap()
        .with_second(0)
        .unwrap();
    set_datetime(&new_datetime).await;
}

pub async fn set_day(day: u32) {
    let current_datetime = get_datetime().await;
    let new_datetime = current_datetime.with_day(day).unwrap();
    set_datetime(&new_datetime).await;
}

pub async fn set_month(month: u32) {
    let current_datetime = get_datetime().await;
    let new_datetime = current_datetime.with_month(month).unwrap();
    set_datetime(&new_datetime).await;
}

async fn set_datetime(datetime: &NaiveDateTime) {
    RTC.lock()
        .await
        .borrow_mut()
        .as_mut()
        .unwrap()
        .0
        .set_datetime(datetime)
        .unwrap();
}
