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

pub async fn get_year() -> i32 {
    let datetime = get_datetime().await;
    datetime.year()
}

pub async fn is_leap_year() -> bool {
    let year = get_year().await;
    year % 4 == 0 && (year % 100 != 0 || (year % 100 == 0 && year % 400 == 0))
}

pub fn is_leap_year_known_year(year: i32) -> bool {
    year % 4 == 0 && (year % 100 != 0 || (year % 100 == 0 && year % 400 == 0))
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

pub async fn set_day(mut day: u32) {
    let current_datetime = get_datetime().await;

    // ensure day does not exceed max day in month
    let max_day = get_max_day_in_month(get_month().await).await;
    if day > max_day {
        day = max_day;
    }

    let new_datetime = current_datetime.with_day(day).unwrap();
    set_datetime(&new_datetime).await;
}

pub async fn set_month(month: u32) {
    let mut current_datetime = get_datetime().await;

    // check that the current day is not greater than what the month allows
    let day = current_datetime.day();
    let max_day = get_max_day_in_month(month).await;
    if day > max_day {
        current_datetime = current_datetime.with_day(max_day).unwrap();
    }

    let new_datetime = current_datetime.with_month(month).unwrap();
    set_datetime(&new_datetime).await;
}

pub async fn set_year(year: i32) {
    let mut current_datetime = get_datetime().await;

    // check for undoing leap year if year becomes not leap year
    if !is_leap_year_known_year(year) {
        if current_datetime.month() == 2 {
            if current_datetime.day() == 29 {
                current_datetime = current_datetime.with_day(28).unwrap();
            }
        }
    }

    let new_datetime = current_datetime.with_year(year).unwrap();
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

pub async fn get_max_day_in_month(month: u32) -> u32 {
    let mut day = MONTH_TABLE
        .iter()
        .find(|y: &&(u32, u32)| y.0 == month)
        .unwrap()
        .1;

    // handle leap year in feb
    if month == 2 {
        if is_leap_year().await {
            day += 1;
        }
    }

    day
}

const MONTH_TABLE: [(u32, u32); 12] = [
    (1, 31),
    (2, 28),
    (3, 31),
    (4, 30),
    (5, 31),
    (6, 30),
    (7, 31),
    (8, 31),
    (9, 30),
    (10, 31),
    (11, 30),
    (12, 31),
];
