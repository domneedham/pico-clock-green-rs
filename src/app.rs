use defmt::info;

use crate::{
    clock::{ClockApp, PomodoroApp},
    display::display_matrix::DISPLAY_MATRIX,
};

pub trait App<'a> {
    fn get_name(&self) -> &'a str;

    async fn start(&self);
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
}

impl<'a> AppSwitcher<'a> {
    pub fn new(clock_app: ClockApp<'a>, pomodoro_app: PomodoroApp<'a>) -> Self {
        Self {
            showing_app_picker: false,
            clock_app,
            pomodoro_app,
            active_app: Apps::ClockAppOption,
        }
    }

    pub async fn show_app_picker(&mut self) {
        self.showing_app_picker = true;

        match self.active_app {
            Apps::ClockAppOption => self.clock_app.stop().await,
            Apps::PomodoroAppOption => self.pomodoro_app.stop().await,
        }

        DISPLAY_MATRIX
            .queue_text(self.clock_app.get_name(), true)
            .await;
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
            Apps::ClockAppOption => self.clock_app.start().await,
            Apps::PomodoroAppOption => self.pomodoro_app.start().await,
        }
    }
}
