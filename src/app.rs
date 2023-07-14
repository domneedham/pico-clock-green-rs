use embassy_executor::Spawner;

use crate::{clock::ClockApp, display::display_matrix::DISPLAY_MATRIX, pomodoro::PomodoroApp};

pub trait App<'a> {
    fn get_name(&self) -> &'a str;

    async fn start(&self, spawner: Spawner);
    async fn stop(&self);
}

#[derive(PartialEq)]
enum Apps {
    ClockAppOption,
    PomodoroAppOption,
}

pub struct AppSwitcher<'a> {
    pub showing_app_picker: bool,
    pub clock_app: ClockApp<'a>,
    pub pomodoro_app: PomodoroApp<'a>,
    active_app: Apps,
    spawner: Spawner,
}

impl<'a> AppSwitcher<'a> {
    pub fn new(spawner: Spawner, clock_app: ClockApp<'a>, pomodoro_app: PomodoroApp<'a>) -> Self {
        Self {
            showing_app_picker: false,
            clock_app,
            pomodoro_app,
            active_app: Apps::ClockAppOption,
            spawner,
        }
    }

    pub async fn show_app_picker(&mut self) {
        self.showing_app_picker = true;

        match self.active_app {
            Apps::ClockAppOption => self.clock_app.stop().await,
            Apps::PomodoroAppOption => self.pomodoro_app.stop().await,
        }

        self.show_next_app().await;
    }

    pub async fn show_next_app(&mut self) {
        match self.active_app {
            Apps::ClockAppOption => {
                DISPLAY_MATRIX
                    .queue_text(self.pomodoro_app.get_name(), true)
                    .await;

                self.active_app = Apps::PomodoroAppOption;
            }
            Apps::PomodoroAppOption => {
                DISPLAY_MATRIX
                    .queue_text(self.clock_app.get_name(), true)
                    .await;

                self.active_app = Apps::ClockAppOption;
            }
        }
    }

    pub async fn show_previous_app(&mut self) {
        match self.active_app {
            Apps::ClockAppOption => {
                DISPLAY_MATRIX
                    .queue_text(self.pomodoro_app.get_name(), true)
                    .await;

                self.active_app = Apps::PomodoroAppOption;
            }
            Apps::PomodoroAppOption => {
                DISPLAY_MATRIX
                    .queue_text(self.clock_app.get_name(), true)
                    .await;

                self.active_app = Apps::ClockAppOption;
            }
        }
    }

    pub async fn app_selected(&mut self) {
        self.showing_app_picker = false;

        match self.active_app {
            Apps::ClockAppOption => self.clock_app.start(self.spawner).await,
            Apps::PomodoroAppOption => self.pomodoro_app.start(self.spawner).await,
        }
    }
}
