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
