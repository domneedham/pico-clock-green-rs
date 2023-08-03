use embassy_executor::Spawner;
use embassy_futures::select::{
    select4, Either4::First, Either4::Fourth, Either4::Second, Either4::Third,
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};

use crate::{
    alarm::AlarmApp,
    buttons::{ButtonPress, BUTTON_ONE_PRESS, BUTTON_THREE_PRESS, BUTTON_TWO_PRESS},
    clock::ClockApp,
    display::display_matrix::DISPLAY_MATRIX,
    pomodoro::PomodoroApp,
    settings::SettingsApp,
    stopwatch::StopwatchApp,
};

/// Named struct for stopping app spawned tasks.
#[derive(Clone)]
pub struct StopAppTasks;

/// Named struct for showing the app switcher from within a task.
pub struct ShowAppSwitcher;

/// Static signal channel for when a task decides to show the app switcher.
pub static SHOW_APP_SWITCHER: Signal<ThreadModeRawMutex, ShowAppSwitcher> = Signal::new();

/// Common trait that all "Apps" should implement.
pub trait App {
    /// The name of the app for use in the app picker.
    fn get_name(&self) -> &str;

    /// Start the app. Spawn any required async tasks.
    async fn start(&mut self, spawner: Spawner);

    /// Stop the app. Save and clean up in here.
    async fn stop(&mut self);

    /// Handle the top button press. This is always just a short press, as long presses are reservered by the AppController.
    async fn button_one_short_press(&mut self, spawner: Spawner);

    /// Handle the middle button press. Can be a short or long press.
    async fn button_two_press(&mut self, press: ButtonPress, spawner: Spawner);

    /// Handle the bottom button press. Can be a short or long press.
    async fn button_three_press(&mut self, press: ButtonPress, spawner: Spawner);
}

/// All apps that can be switched too.
#[derive(PartialEq)]
enum Apps {
    /// The clock app.
    Clock,

    /// The pomodoro app.
    Pomodoro,

    /// The stopwatch app.
    Stopwatch,

    /// The alarm app.
    Alarm,

    /// The settings app.
    Settings,
}

/// App controller is responsible for managing apps by:
/// - Starting and stopping apps on user selection
/// - Forwarding button presses to active apps
/// - Handling the app switcher
///
/// It contains the main loop on the main core.
pub struct AppController {
    /// The current active app. This should be the one set even if viewing from the app switcher.
    active_app: Apps,

    /// Showing the app picker or not.
    showing_app_picker: bool,

    /// Clock app.
    clock_app: ClockApp,

    /// Pomodoro app.
    pomodoro_app: PomodoroApp,

    /// Stopwatch app.
    stopwatch_app: StopwatchApp,

    /// Alarm app.
    alarm_app: AlarmApp,

    /// Settings app.
    settings_app: SettingsApp,

    /// Embassy spawner so apps can spawn their own background tasks.
    spawner: Spawner,
}

impl AppController {
    /// Create a new app controller. Will take ownership of all apps.
    pub fn new(
        spawner: Spawner,
        clock_app: ClockApp,
        pomodoro_app: PomodoroApp,
        stopwatch_app: StopwatchApp,
        alarm_app: AlarmApp,
        settings_app: SettingsApp,
    ) -> Self {
        Self {
            active_app: Apps::Clock,
            showing_app_picker: false,
            clock_app,
            pomodoro_app,
            stopwatch_app,
            alarm_app,
            settings_app,
            spawner,
        }
    }

    /// The main program loop.
    pub async fn run_forever(&mut self) -> ! {
        self.app_selected().await;

        loop {
            let t = select4(
                SHOW_APP_SWITCHER.wait(),
                BUTTON_ONE_PRESS.wait(),
                BUTTON_TWO_PRESS.wait(),
                BUTTON_THREE_PRESS.wait(),
            )
            .await;

            match t {
                First(_) => self.show_app_picker().await,
                Second(press) => self.button_one_press(press).await,
                Third(press) => self.button_two_press(press).await,
                Fourth(press) => self.button_three_press(press).await,
            }
        }
    }

    /// Handle the top button press when signaled from the button module.
    pub async fn button_one_press(&mut self, press: ButtonPress) {
        match press {
            ButtonPress::Short => {
                if self.showing_app_picker {
                    self.app_selected().await;
                } else {
                    match self.active_app {
                        Apps::Clock => self.clock_app.button_one_short_press(self.spawner).await,
                        Apps::Pomodoro => {
                            self.pomodoro_app.button_one_short_press(self.spawner).await
                        }
                        Apps::Stopwatch => {
                            self.stopwatch_app
                                .button_one_short_press(self.spawner)
                                .await
                        }
                        Apps::Alarm => self.alarm_app.button_one_short_press(self.spawner).await,
                        Apps::Settings => {
                            self.settings_app.button_one_short_press(self.spawner).await
                        }
                    }
                }
            }
            ButtonPress::Long => self.show_app_picker().await,
            ButtonPress::Double => {}
        };
    }

    /// Handle the middle button press when signaled from the button module.
    pub async fn button_two_press(&mut self, press: ButtonPress) {
        if self.showing_app_picker {
            self.show_next_app().await;
            return;
        }

        match self.active_app {
            Apps::Clock => self.clock_app.button_two_press(press, self.spawner).await,
            Apps::Pomodoro => {
                self.pomodoro_app
                    .button_two_press(press, self.spawner)
                    .await
            }
            Apps::Stopwatch => {
                self.stopwatch_app
                    .button_two_press(press, self.spawner)
                    .await
            }
            Apps::Alarm => self.alarm_app.button_two_press(press, self.spawner).await,
            Apps::Settings => {
                self.settings_app
                    .button_two_press(press, self.spawner)
                    .await
            }
        };
    }

    /// Handle the bottom button press when signaled from the button module.
    pub async fn button_three_press(&mut self, press: ButtonPress) {
        if self.showing_app_picker {
            self.show_previous_app().await;
            return;
        }

        match self.active_app {
            Apps::Clock => self.clock_app.button_three_press(press, self.spawner).await,
            Apps::Pomodoro => {
                self.pomodoro_app
                    .button_three_press(press, self.spawner)
                    .await
            }
            Apps::Stopwatch => {
                self.stopwatch_app
                    .button_three_press(press, self.spawner)
                    .await
            }
            Apps::Alarm => self.alarm_app.button_three_press(press, self.spawner).await,
            Apps::Settings => {
                self.settings_app
                    .button_three_press(press, self.spawner)
                    .await
            }
        };
    }

    /// Show the app picker. Must stop the active app first to allow it to clean up.
    async fn show_app_picker(&mut self) {
        self.showing_app_picker = true;

        match self.active_app {
            Apps::Clock => self.clock_app.stop().await,
            Apps::Pomodoro => self.pomodoro_app.stop().await,
            Apps::Stopwatch => self.stopwatch_app.stop().await,
            Apps::Alarm => self.alarm_app.stop().await,
            Apps::Settings => self.settings_app.stop().await,
        }

        critical_section::with(|cs| {
            DISPLAY_MATRIX.clear_all(cs, true);
        });

        self.show_next_app().await;
    }

    /// Show the next app text in the display.
    async fn show_next_app(&mut self) {
        match self.active_app {
            Apps::Clock => {
                DISPLAY_MATRIX
                    .queue_text(self.pomodoro_app.get_name(), 1000, true, false)
                    .await;

                self.active_app = Apps::Pomodoro;
            }
            Apps::Pomodoro => {
                DISPLAY_MATRIX
                    .queue_text(self.stopwatch_app.get_name(), 1000, true, false)
                    .await;

                self.active_app = Apps::Stopwatch;
            }
            Apps::Stopwatch => {
                DISPLAY_MATRIX
                    .queue_text(self.alarm_app.get_name(), 1000, true, false)
                    .await;

                self.active_app = Apps::Alarm;
            }
            Apps::Alarm => {
                DISPLAY_MATRIX
                    .queue_text(self.settings_app.get_name(), 1000, true, false)
                    .await;

                self.active_app = Apps::Settings;
            }
            Apps::Settings => {
                DISPLAY_MATRIX
                    .queue_text(self.clock_app.get_name(), 1000, true, false)
                    .await;

                self.active_app = Apps::Clock;
            }
        }
    }

    /// Show the previous app text in the display.
    async fn show_previous_app(&mut self) {
        match self.active_app {
            Apps::Clock => {
                DISPLAY_MATRIX
                    .queue_text(self.settings_app.get_name(), 1000, true, false)
                    .await;

                self.active_app = Apps::Settings;
            }
            Apps::Pomodoro => {
                DISPLAY_MATRIX
                    .queue_text(self.clock_app.get_name(), 1000, true, false)
                    .await;

                self.active_app = Apps::Clock;
            }
            Apps::Stopwatch => {
                DISPLAY_MATRIX
                    .queue_text(self.pomodoro_app.get_name(), 1000, true, false)
                    .await;

                self.active_app = Apps::Pomodoro;
            }
            Apps::Alarm => {
                DISPLAY_MATRIX
                    .queue_text(self.stopwatch_app.get_name(), 1000, true, false)
                    .await;

                self.active_app = Apps::Stopwatch;
            }
            Apps::Settings => {
                DISPLAY_MATRIX
                    .queue_text(self.alarm_app.get_name(), 1000, true, false)
                    .await;

                self.active_app = Apps::Alarm;
            }
        }
    }

    /// Dismiss the app picker and start the active app.
    async fn app_selected(&mut self) {
        self.showing_app_picker = false;

        match self.active_app {
            Apps::Clock => self.clock_app.start(self.spawner).await,
            Apps::Pomodoro => self.pomodoro_app.start(self.spawner).await,
            Apps::Stopwatch => self.stopwatch_app.start(self.spawner).await,
            Apps::Alarm => self.alarm_app.start(self.spawner).await,
            Apps::Settings => self.settings_app.start(self.spawner).await,
        }
    }
}
