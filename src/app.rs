use embassy_executor::Spawner;
use embassy_futures::select::{select3, Either3::First, Either3::Second, Either3::Third};

use crate::{
    buttons::{ButtonPress, BUTTON_ONE_PRESS, BUTTON_THREE_PRESS, BUTTON_TWO_PRESS},
    clock::ClockApp,
    display::display_matrix::DISPLAY_MATRIX,
    pomodoro::PomodoroApp,
    settings::SettingsApp,
};

#[derive(Clone)]
pub struct StopAppTasks();

pub trait App<'a> {
    fn get_name(&self) -> &'a str;

    async fn start(&self, spawner: Spawner);
    async fn stop(&self);

    async fn button_one_short_press(&self, spawner: Spawner);
    async fn button_two_press(&self, press: ButtonPress, spawner: Spawner);
    async fn button_three_press(&self, press: ButtonPress, spawner: Spawner);
}

#[derive(PartialEq)]
enum Apps {
    ClockApp,
    PomodoroApp,
    SettingsApp,
}

pub struct AppController<'a> {
    pub showing_app_picker: bool,
    pub clock_app: ClockApp<'a>,
    pub pomodoro_app: PomodoroApp<'a>,
    pub settings_app: SettingsApp<'a>,
    active_app: Apps,
    spawner: Spawner,
}

impl<'a> AppController<'a> {
    pub fn new(
        spawner: Spawner,
        clock_app: ClockApp<'a>,
        pomodoro_app: PomodoroApp<'a>,
        settings_app: SettingsApp<'a>,
    ) -> Self {
        Self {
            showing_app_picker: false,
            clock_app,
            pomodoro_app,
            settings_app,
            active_app: Apps::ClockApp,
            spawner,
        }
    }

    pub async fn run_forever(&mut self) -> ! {
        self.app_selected().await;

        loop {
            let button_press = select3(
                BUTTON_ONE_PRESS.wait(),
                BUTTON_TWO_PRESS.wait(),
                BUTTON_THREE_PRESS.wait(),
            )
            .await;

            match button_press {
                First(press) => self.button_one_press(press).await,
                Second(press) => self.button_two_press(press).await,
                Third(press) => self.button_three_press(press).await,
            }
        }
    }

    pub async fn button_one_press(&mut self, press: ButtonPress) {
        match press {
            ButtonPress::ShortPress => {
                if self.showing_app_picker {
                    self.app_selected().await;
                } else {
                    match self.active_app {
                        Apps::ClockApp => self.clock_app.button_one_short_press(self.spawner).await,
                        Apps::PomodoroApp => {
                            self.pomodoro_app.button_one_short_press(self.spawner).await
                        }
                        Apps::SettingsApp => {
                            self.settings_app.button_one_short_press(self.spawner).await
                        }
                    }
                }
            }
            ButtonPress::LongPress => self.show_app_picker().await,
        };
    }

    pub async fn button_two_press(&mut self, press: ButtonPress) {
        if self.showing_app_picker {
            self.show_next_app().await;
            return;
        }

        match self.active_app {
            Apps::ClockApp => self.clock_app.button_two_press(press, self.spawner).await,
            Apps::PomodoroApp => {
                self.pomodoro_app
                    .button_two_press(press, self.spawner)
                    .await
            }
            Apps::SettingsApp => {
                self.settings_app
                    .button_two_press(press, self.spawner)
                    .await
            }
        };
    }

    pub async fn button_three_press(&mut self, press: ButtonPress) {
        if self.showing_app_picker {
            self.show_previous_app().await;
            return;
        }

        match self.active_app {
            Apps::ClockApp => self.clock_app.button_three_press(press, self.spawner).await,
            Apps::PomodoroApp => {
                self.pomodoro_app
                    .button_three_press(press, self.spawner)
                    .await
            }
            Apps::SettingsApp => {
                self.settings_app
                    .button_three_press(press, self.spawner)
                    .await
            }
        };
    }

    async fn show_app_picker(&mut self) {
        self.showing_app_picker = true;

        match self.active_app {
            Apps::ClockApp => self.clock_app.stop().await,
            Apps::PomodoroApp => self.pomodoro_app.stop().await,
            Apps::SettingsApp => self.settings_app.stop().await,
        }

        self.show_next_app().await;
    }

    async fn show_next_app(&mut self) {
        match self.active_app {
            Apps::ClockApp => {
                DISPLAY_MATRIX
                    .queue_text(self.pomodoro_app.get_name(), true)
                    .await;

                self.active_app = Apps::PomodoroApp;
            }
            Apps::PomodoroApp => {
                DISPLAY_MATRIX
                    .queue_text(self.settings_app.get_name(), true)
                    .await;

                self.active_app = Apps::SettingsApp;
            }
            Apps::SettingsApp => {
                DISPLAY_MATRIX
                    .queue_text(self.clock_app.get_name(), true)
                    .await;

                self.active_app = Apps::ClockApp;
            }
        }
    }

    async fn show_previous_app(&mut self) {
        match self.active_app {
            Apps::ClockApp => {
                DISPLAY_MATRIX
                    .queue_text(self.settings_app.get_name(), true)
                    .await;

                self.active_app = Apps::SettingsApp;
            }
            Apps::PomodoroApp => {
                DISPLAY_MATRIX
                    .queue_text(self.clock_app.get_name(), true)
                    .await;

                self.active_app = Apps::ClockApp;
            }
            Apps::SettingsApp => {
                DISPLAY_MATRIX
                    .queue_text(self.pomodoro_app.get_name(), true)
                    .await;

                self.active_app = Apps::PomodoroApp;
            }
        }
    }

    async fn app_selected(&mut self) {
        self.showing_app_picker = false;

        match self.active_app {
            Apps::ClockApp => self.clock_app.start(self.spawner).await,
            Apps::PomodoroApp => self.pomodoro_app.start(self.spawner).await,
            Apps::SettingsApp => self.settings_app.start(self.spawner).await,
        }
    }
}
