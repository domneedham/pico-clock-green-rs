use crate::{app::App, display::display_matrix::DISPLAY_MATRIX};

#[derive(PartialEq)]
pub struct ClockApp<'a> {
    pub name: &'a str,
}

impl<'a> App<'a> for ClockApp<'a> {
    fn get_name(&self) -> &'a str {
        self.name
    }

    async fn start(&self) {
        DISPLAY_MATRIX.queue_text("21:21", true).await;
    }

    async fn stop(&self) {
        // do nothing yet.
    }
}

#[derive(PartialEq)]
pub struct PomodoroApp<'a> {
    pub name: &'a str,
}

impl<'a> App<'a> for PomodoroApp<'a> {
    fn get_name(&self) -> &'a str {
        self.name
    }

    async fn start(&self) {
        DISPLAY_MATRIX.queue_text("POMO", true).await;
    }

    async fn stop(&self) {
        // do nothing yet.
    }
}
