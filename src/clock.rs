use ds323x::{Datelike, Timelike};
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either::First, Either::Second};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, pubsub::PubSubChannel};
use embassy_time::{Duration, Timer};

use crate::{
    app::{App, StopAppTasks},
    buttons::ButtonPress,
    config::{self, TimePreference},
    display::display_matrix::{TimeColon, DISPLAY_MATRIX},
    rtc::{self},
    speaker, temperature,
};

/// Channel for firing events of when tasks should be stopped.
static PUB_SUB_CHANNEL: PubSubChannel<ThreadModeRawMutex, StopAppTasks, 1, 1, 1> =
    PubSubChannel::new();

/// Clock app.
/// Will show the current time on the display.
pub struct ClockApp {}

impl ClockApp {
    /// Create a new clock app.
    pub fn new() -> Self {
        Self {}
    }
}

impl App for ClockApp {
    fn get_name(&self) -> &str {
        "Clock"
    }

    async fn start(&mut self, spawner: Spawner) {
        self.start_clock(spawner).await;
    }

    async fn stop(&mut self) {
        self.cancel_clock();
    }

    async fn button_one_short_press(&mut self, _: Spawner) {}

    async fn button_two_press(&mut self, press: ButtonPress, _: Spawner) {
        match press {
            ButtonPress::Short => {
                show_temperature().await;
            }
            ButtonPress::Long => {
                config::CONFIG
                    .lock()
                    .await
                    .borrow_mut()
                    .toggle_temperature_preference();

                let temp_pref = config::CONFIG
                    .lock()
                    .await
                    .borrow()
                    .get_temperature_preference();
                DISPLAY_MATRIX.show_temperature_icon(temp_pref);
            }
            ButtonPress::Double => {
                config::CONFIG
                    .lock()
                    .await
                    .borrow_mut()
                    .toggle_time_preference();

                let time_pref = config::CONFIG.lock().await.borrow().get_time_preference();
                let datetime = rtc::get_datetime().await;
                DISPLAY_MATRIX.show_time_icon(time_pref, datetime.hour());
            }
        }
    }

    async fn button_three_press(&mut self, _: ButtonPress, _: Spawner) {}
}

impl ClockApp {
    /// Start the clock background task.
    async fn start_clock(&self, spawner: Spawner) {
        // try to start the clock, but wait if the spawner is busy and retry
        loop {
            let res = spawner.spawn(clock());
            match res {
                Ok(_) => break,
                Err(_) => Timer::after(Duration::from_millis(100)).await,
            }
        }
    }

    /// Cancel the clock background task.
    fn cancel_clock(&self) {
        PUB_SUB_CHANNEL
            .immediate_publisher()
            .publish_immediate(StopAppTasks);
    }
}

/// The clock background task. Shows the current time and appropriate icons for AM/PM and day of week.
///
/// Will continue to run until signalled not too.
#[embassy_executor::task]
async fn clock() {
    let mut sub = PUB_SUB_CHANNEL.subscriber().unwrap();

    let datetime = rtc::get_datetime().await;
    let mut last_hour = datetime.hour();
    let mut last_min = datetime.minute();
    let mut last_day = datetime.weekday();

    show_time(last_hour, last_min, TimeColon::Full, true).await;

    DISPLAY_MATRIX.show_day_icon(last_day);

    let time_pref = config::CONFIG.lock().await.borrow().get_time_preference();
    DISPLAY_MATRIX.show_time_icon(time_pref, last_hour);

    let should_hourly_ring = config::CONFIG.lock().await.borrow().get_hourly_ring();
    if should_hourly_ring {
        DISPLAY_MATRIX.show_icon("Hourly");
    }

    let should_scroll_temp = config::CONFIG.lock().await.borrow().get_auto_scroll_temp();
    if should_scroll_temp {
        DISPLAY_MATRIX.show_icon("MoveOn");
    }

    let temp_pref = temperature::get_temperature_preference().await;
    DISPLAY_MATRIX.show_temperature_icon(temp_pref);

    loop {
        let res = select(sub.next_message(), Timer::after(Duration::from_secs(1))).await;

        match res {
            First(_) => break,
            Second(_) => {
                let datetime = rtc::get_datetime().await;

                let hour = datetime.hour();
                let min = datetime.minute();
                let second = datetime.second();

                if second % 2 == 0 {
                    if second > 30 && second < 45 {
                        show_time(hour, min, TimeColon::Top, false).await;
                    } else {
                        show_time(hour, min, TimeColon::Empty, false).await;
                    }
                } else {
                    if second < 15 {
                        show_time(hour, min, TimeColon::Top, false).await;
                    } else if second < 30 {
                        show_time(hour, min, TimeColon::Bottom, false).await;
                    } else if second < 45 {
                        show_time(hour, min, TimeColon::Bottom, false).await;
                    } else {
                        show_time(hour, min, TimeColon::Full, false).await;
                    }
                }

                if hour != last_hour || min != last_min {
                    if hour != last_hour {
                        if hour == 0 || hour == 12 {
                            let time_pref =
                                config::CONFIG.lock().await.borrow().get_time_preference();
                            DISPLAY_MATRIX.show_time_icon(time_pref, hour);
                        }

                        if should_hourly_ring {
                            speaker::sound(speaker::SoundType::ShortBeep);
                        }
                    }

                    last_hour = hour;
                    last_min = min;
                }

                let day = datetime.weekday();
                if day != last_day {
                    DISPLAY_MATRIX.show_day_icon(day);
                    last_day = day;
                }

                if min % 5 == 0 && second == 25 && should_scroll_temp {
                    let temp_pref = temperature::get_temperature_preference().await;
                    let temp = temperature::get_temperature_off_preference().await;

                    let mut hour = hour;
                    let pref = config::CONFIG.lock().await.borrow().get_time_preference();
                    if let TimePreference::Twelve = pref {
                        hour = convert_24_to_12(hour);
                    }

                    DISPLAY_MATRIX
                        .queue_time_temperature(hour, min, temp, temp_pref, false)
                        .await;
                }
            }
        }
    }
}

/// Show the temperature.
async fn show_temperature() {
    let temp_pref = temperature::get_temperature_preference().await;
    let temp = temperature::get_temperature_off_preference().await;
    // show temperature (holds for 5 seconds) and then show time again
    DISPLAY_MATRIX
        .queue_temperature(temp, temp_pref, false, false)
        .await;
}

/// Show the time.
async fn show_time(mut hour: u32, minute: u32, colon: TimeColon, show_now: bool) {
    let pref = config::CONFIG.lock().await.borrow().get_time_preference();

    if let TimePreference::Twelve = pref {
        hour = convert_24_to_12(hour);
    }

    DISPLAY_MATRIX
        .queue_time(hour, minute, colon, 0, show_now, false)
        .await;
}

/// Convert 24hr time into 12hr time.
fn convert_24_to_12(hour: u32) -> u32 {
    if hour <= 12 {
        hour
    } else if hour == 13 {
        1
    } else if hour == 14 {
        2
    } else if hour == 15 {
        3
    } else if hour == 16 {
        4
    } else if hour == 17 {
        5
    } else if hour == 18 {
        6
    } else if hour == 19 {
        7
    } else if hour == 20 {
        8
    } else if hour == 21 {
        9
    } else if hour == 22 {
        10
    } else {
        11
    }
}
