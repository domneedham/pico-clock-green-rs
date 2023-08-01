use chrono::{Datelike, NaiveDateTime, Timelike};
use core::cell::RefCell;
use ds323x::{DateTimeAccess, Ds323x};
use embassy_rp::{i2c, peripherals::I2C1};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex};

/// Wrapper around the Ds323x crate for the Ds3231 used in the pico clock.
pub struct Ds3231<'a>(
    pub  Ds323x<
        ds323x::interface::I2cInterface<i2c::I2c<'a, I2C1, i2c::Blocking>>,
        ds323x::ic::DS3231,
    >,
);

/// Static reference to the Ds3231.
///
/// **Init must be called first to set the value, or it will return None.**
static RTC: Mutex<ThreadModeRawMutex, RefCell<Option<Ds3231>>> = Mutex::new(RefCell::new(None));

/// Initialise the static RTC value.
pub async fn init(ds3231: Ds3231<'static>) {
    RTC.lock().await.replace(Some(ds3231));
}

/// Get the current datetime from the RTC.
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

/// Get the current hour from the RTC.
pub async fn get_hour() -> u32 {
    let datetime = get_datetime().await;
    datetime.hour()
}

/// Get the current minute from the RTC.
pub async fn get_minute() -> u32 {
    let datetime = get_datetime().await;
    datetime.minute()
}

/// Get the current day from the RTC.
pub async fn get_day() -> u32 {
    let datetime = get_datetime().await;
    datetime.day()
}

/// Get the current month from the RTC.
pub async fn get_month() -> u32 {
    let datetime = get_datetime().await;
    datetime.month()
}

/// Get the current year from the RTC.
pub async fn get_year() -> i32 {
    let datetime = get_datetime().await;
    datetime.year()
}

/// Determine if it is a leap year based on the current value in RTC.
pub async fn is_leap_year() -> bool {
    let year = get_year().await;
    is_leap_year_opt(year)
}

/// Determine if the passed year is a leap year.
pub fn is_leap_year_opt(year: i32) -> bool {
    (year % 400 == 0 || year % 100 != 0) && year % 4 == 0
}

/// Set the passed hour into the RTC.
///
/// This will set the seconds to 0.
pub async fn set_hour(hour: u32) {
    let current_datetime = get_datetime().await;
    let new_datetime = current_datetime
        .with_hour(hour)
        .unwrap()
        .with_second(0)
        .unwrap();
    set_datetime(&new_datetime).await;
}

/// Set the passed minute into the RTC.
///
/// This will set the seconds to 0.
pub async fn set_minute(minute: u32) {
    let current_datetime = get_datetime().await;
    let new_datetime = current_datetime
        .with_minute(minute)
        .unwrap()
        .with_second(0)
        .unwrap();
    set_datetime(&new_datetime).await;
}

/// Set the day into the RTC.
///
/// It will automatically handle larger than allowed days by setting the value to the maximum allowed for the current month in the RTC.
///
/// For example, setting February 29th on a non leap year will become 28th February.
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

/// Set the month into the RTC.
///
/// It will automatically handle larger than allowed days by setting the value to the maximum allowed for the month passed.
///
/// For example, changing from 31st December into February will become 28th February.
/// Note how leap years in this example are not handled, this will need to be done seperately.
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

/// Set the year into the RTC.
///
/// It will automatically handle converting from a leap year to non leap year as required.
///
/// For example, going from 29th February 2024 to 29th February 2025 will become 28th February 2025.
pub async fn set_year(year: i32) {
    let mut current_datetime = get_datetime().await;

    // check for undoing leap year if year becomes not leap year
    if !is_leap_year_opt(year) && current_datetime.month() == 2 && current_datetime.day() == 29 {
        current_datetime = current_datetime.with_day(28).unwrap();
    }

    let new_datetime = current_datetime.with_year(year).unwrap();
    set_datetime(&new_datetime).await;
}

/// Replace the datetime in the RTC with the passed datetime.
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

/// Get the maximum possible day in the passed month.
///
/// It will automatically handle leap years by adding a 1 to the February motnh.
pub async fn get_max_day_in_month(month: u32) -> u32 {
    let mut day = MONTH_TABLE
        .iter()
        .find(|y: &&(u32, u32)| y.0 == month)
        .unwrap()
        .1;

    // handle leap year in feb
    if month == 2 && is_leap_year().await {
        day += 1;
    }

    day
}

/// Days in month lookup table.
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

/// All temperature related functionality.
pub mod temperature {
    use super::*;

    /// Get the current temperature from RTC.
    pub async fn get_temperature() -> f32 {
        RTC.lock()
            .await
            .borrow_mut()
            .as_mut()
            .unwrap()
            .0
            .temperature()
            .unwrap()
    }
}
