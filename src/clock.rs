use defmt::info;
use embassy_executor::Spawner;
use embassy_futures::select::{select, Either::First, Either::Second};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, pubsub::PubSubChannel};
use embassy_time::{Duration, Timer};

use crate::{
    app::{App, StopAppTasks},
    display::display_matrix::DISPLAY_MATRIX,
};

static PUB_SUB_CHANNEL: PubSubChannel<ThreadModeRawMutex, StopAppTasks, 1, 1, 1> =
    PubSubChannel::new();

pub struct ClockApp<'a> {
    name: &'a str,
}

impl<'a> ClockApp<'a> {
    pub fn new(name: &'a str) -> Self {
        Self { name }
    }
}
impl<'a> App<'a> for ClockApp<'a> {
    fn get_name(&self) -> &'a str {
        self.name
    }

    async fn start(&self, spawner: Spawner) {
        DISPLAY_MATRIX.queue_text("21:21", true).await;
        spawner.spawn(clock()).unwrap();
    }

    async fn stop(&self) {
        PUB_SUB_CHANNEL
            .immediate_publisher()
            .publish_immediate(StopAppTasks());
    }

    async fn button_one_short_press(&self) {
        DISPLAY_MATRIX.test_text().await;

        critical_section::with(|cs| {
            DISPLAY_MATRIX.test_icons(cs);
        });
    }

    async fn button_two_press(&self, _: crate::buttons::ButtonPress) {
        critical_section::with(|cs| {
            DISPLAY_MATRIX.clear_all(cs, true);
        });
    }

    async fn button_three_press(&self, _: crate::buttons::ButtonPress) {
        critical_section::with(|cs| {
            DISPLAY_MATRIX.fill_all(cs, true);
        });
    }
}

#[embassy_executor::task]
async fn clock() {
    let mut sub = PUB_SUB_CHANNEL.subscriber().unwrap();
    loop {
        let res = select(sub.next_message(), Timer::after(Duration::from_secs(1))).await;

        match res {
            First(_) => break,
            Second(_) => info!("Would update time"),
        }
    }
}
