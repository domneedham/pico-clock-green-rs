use ds323x::{Datelike, Timelike};
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either::First, Either::Second};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, pubsub::PubSubChannel};
use embassy_time::{Duration, Timer};

use crate::{
    app::{App, StopAppTasks},
    buttons::ButtonPress,
    config,
    display::display_matrix::DISPLAY_MATRIX,
    rtc::{self},
    speaker,
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

    async fn button_one_short_press(&mut self, spawner: Spawner) {
        self.cancel_clock();
        DISPLAY_MATRIX
            .queue_text("CLOCK INTERRUPT", 1000, true)
            .await;
        self.start_clock(spawner).await;
    }

    async fn button_two_press(&mut self, _: ButtonPress, _: Spawner) {
        critical_section::with(|cs| {
            DISPLAY_MATRIX.clear_all(cs, true);
        });
    }

    async fn button_three_press(&mut self, _: ButtonPress, _: Spawner) {
        critical_section::with(|cs| {
            DISPLAY_MATRIX.fill_all(cs, true);
        });
    }
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

    DISPLAY_MATRIX
        .queue_time(last_hour, last_min, 1000, false)
        .await;

    if last_hour >= 12 {
        DISPLAY_MATRIX.hide_icon("AM");
        DISPLAY_MATRIX.show_icon("PM");
    } else {
        DISPLAY_MATRIX.hide_icon("PM");
        DISPLAY_MATRIX.show_icon("AM");
    }

    DISPLAY_MATRIX.show_day_icon(last_day);

    let should_hourly_ring = config::CONFIG.lock().await.borrow().get_hourly_ring();
    if should_hourly_ring {
        DISPLAY_MATRIX.show_icon("Hourly");
    }

    loop {
        let res = select(sub.next_message(), Timer::after(Duration::from_secs(1))).await;

        match res {
            First(_) => break,
            Second(_) => {
                let datetime = rtc::get_datetime().await;

                let hour = datetime.hour();
                let min = datetime.minute();
                if hour != last_hour || min != last_min {
                    DISPLAY_MATRIX.queue_time(hour, min, 1000, false).await;

                    if hour >= 12 {
                        DISPLAY_MATRIX.hide_icon("AM");
                        DISPLAY_MATRIX.show_icon("PM");
                    } else {
                        DISPLAY_MATRIX.hide_icon("PM");
                        DISPLAY_MATRIX.show_icon("AM");
                    }

                    if hour != last_hour && should_hourly_ring {
                        speaker::sound(speaker::SoundType::ShortBeep);
                    }

                    last_hour = hour;
                    last_min = min;
                }

                let day = datetime.weekday();
                if day != last_day {
                    DISPLAY_MATRIX.show_day_icon(day);
                    last_day = day;
                }
            }
        }
    }
}
