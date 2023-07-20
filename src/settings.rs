use embassy_executor::Spawner;

use crate::{app::App, buttons::ButtonPress, display::display_matrix::DISPLAY_MATRIX};

pub struct SettingsApp<'a> {
    name: &'a str,
}

impl<'a> SettingsApp<'a> {
    pub fn new(name: &'a str) -> Self {
        Self { name }
    }
}

impl<'a> App<'a> for SettingsApp<'a> {
    fn get_name(&self) -> &'a str {
        self.name
    }

    async fn start(&self, _: Spawner) {
        DISPLAY_MATRIX.queue_text("SETTINGS", true).await;
    }

    async fn stop(&self) {}

    async fn button_one_short_press(&self, _: Spawner) {
        DISPLAY_MATRIX.queue_text("SETTINGS INTERRUPT", true).await;
    }

    async fn button_two_press(&self, _: ButtonPress, _: Spawner) {}

    async fn button_three_press(&self, _: ButtonPress, _: Spawner) {}
}
