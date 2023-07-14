use defmt::info;
use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, pubsub::PubSubChannel};
use embassy_time::{Duration, Timer};

use crate::{
    app::{App, StopAppTasks},
    display::display_matrix::DISPLAY_MATRIX,
};

static PUB_SUB_CHANNEL: PubSubChannel<ThreadModeRawMutex, StopAppTasks, 1, 1, 1> =
    PubSubChannel::new();

#[derive(PartialEq)]
pub struct ClockApp<'a> {
    pub name: &'a str,
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
}

#[embassy_executor::task]
async fn clock() {
    let mut sub = PUB_SUB_CHANNEL.subscriber().unwrap();
    loop {
        let res = sub.try_next_message();
        if res.is_some() {
            break;
        }

        info!("Would update time");

        Timer::after(Duration::from_secs(1)).await;
    }
}
