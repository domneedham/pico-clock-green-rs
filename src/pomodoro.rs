use embassy_executor::Spawner;

use crate::{app::App, buttons::ButtonPress, display::display_matrix::DISPLAY_MATRIX};

pub struct PomodoroApp<'a> {
    pub name: &'a str,
}

impl<'a> PomodoroApp<'a> {
    pub fn new(name: &'a str) -> Self {
        Self { name }
    }
}

impl<'a> App<'a> for PomodoroApp<'a> {
    fn get_name(&self) -> &'a str {
        self.name
    }

    async fn start(&mut self, _: Spawner) {
        DISPLAY_MATRIX.queue_text("POMO", true).await;
    }

    async fn stop(&mut self) {
        // do nothing yet.
    }

    async fn button_one_short_press(&mut self, _: Spawner) {
        DISPLAY_MATRIX.queue_text("POMO INTERRUPT", true).await;
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
