use embassy_executor::Spawner;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, pubsub::PubSubChannel};

use crate::{
    app::{App, StopAppTasks},
    buttons::ButtonPress,
    display::display_matrix::DISPLAY_MATRIX,
};

/// Channel for firing events of when tasks should be stopped.
static PUB_SUB_CHANNEL: PubSubChannel<ThreadModeRawMutex, StopAppTasks, 1, 1, 1> =
    PubSubChannel::new();

/// Alarm app.
/// Used for configuring alarms.
pub struct AlarmApp {}

impl AlarmApp {
    /// Create a new clock app.
    pub fn new() -> Self {
        Self {}
    }
}

impl App for AlarmApp {
    fn get_name(&self) -> &str {
        "Alarms"
    }

    async fn start(&mut self, _: Spawner) {
        DISPLAY_MATRIX.queue_text("Alarm", 1000, true, false).await;
    }

    async fn stop(&mut self) {
        PUB_SUB_CHANNEL
            .immediate_publisher()
            .publish_immediate(StopAppTasks);
    }

    async fn button_one_short_press(&mut self, _: Spawner) {}

    async fn button_two_press(&mut self, _: ButtonPress, _: Spawner) {}

    async fn button_three_press(&mut self, _: ButtonPress, _: Spawner) {}
}
