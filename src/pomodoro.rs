use defmt::info;
use embassy_executor::Spawner;

use crate::{app::App, display::display_matrix::DISPLAY_MATRIX};

#[derive(PartialEq)]
pub struct PomodoroApp<'a> {
    pub name: &'a str,
}

impl<'a> App<'a> for PomodoroApp<'a> {
    fn get_name(&self) -> &'a str {
        self.name
    }

    async fn start(&self, _: Spawner) {
        DISPLAY_MATRIX.queue_text("POMO", true).await;
    }

    async fn stop(&self) {
        // do nothing yet.
    }

    async fn button_one_short_press(&self) {
        info!("POMO");
    }

    async fn button_two_press(&self, press: crate::buttons::ButtonPress) {
        info!("POMO");
    }

    async fn button_three_press(&self, press: crate::buttons::ButtonPress) {
        info!("POMO");
    }
}
