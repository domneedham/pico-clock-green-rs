use embassy_executor::Spawner;

use crate::{app::App, buttons::ButtonPress, display::display_matrix::DISPLAY_MATRIX};

enum SettingsConfig {
    Hour,
    Minute,
    Day,
    Month,
    Year,
}

pub struct SettingsApp<'a> {
    name: &'a str,
    active_config: SettingsConfig,
}

impl<'a> SettingsApp<'a> {
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            active_config: SettingsConfig::Hour,
        }
    }
}

impl<'a> App<'a> for SettingsApp<'a> {
    fn get_name(&self) -> &'a str {
        self.name
    }

    async fn start(&mut self, _: Spawner) {
        self.active_config = SettingsConfig::Hour;
        DISPLAY_MATRIX.queue_text("SETTINGS", true).await;
    }

    async fn stop(&mut self) {}

    async fn button_one_short_press(&mut self, _: Spawner) {
        DISPLAY_MATRIX.queue_text("SETTINGS INTERRUPT", true).await;
    }

    async fn button_two_press(&mut self, _: ButtonPress, _: Spawner) {}

    async fn button_three_press(&mut self, _: ButtonPress, _: Spawner) {}
}
