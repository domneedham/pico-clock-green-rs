use embassy_executor::Spawner;
use embassy_futures::select::{select3, Either3::*};
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex, pubsub::PubSubChannel, signal::Signal,
};

use embassy_time::{Duration, Timer};

use crate::{
    app::{App, StopAppTasks},
    buttons::ButtonPress,
    display::display_matrix::{TimeColon, DISPLAY_MATRIX},
};

use self::configurations::AlarmConfiguration;

pub enum AlarmNumber {
    One,
    Two,
}

/// Alarm app.
/// Used for configuring alarms.
pub struct AlarmApp {
    alarm_one: AlarmConfiguration,
    alarm_two: AlarmConfiguration,
    active_alarm: AlarmNumber,
    in_menu: bool,
}

impl AlarmApp {
    /// Create a new clock app.
    pub fn new() -> Self {
        Self {
            alarm_one: AlarmConfiguration::new(AlarmNumber::One),
            alarm_two: AlarmConfiguration::new(AlarmNumber::Two),
            active_alarm: AlarmNumber::One,
            in_menu: true,
        }
    }

    pub async fn show(&self) {
        match self.active_alarm {
            AlarmNumber::One => {
                DISPLAY_MATRIX
                    .queue_text(self.alarm_one.get_name(), 0, true, false)
                    .await
            }
            AlarmNumber::Two => {
                DISPLAY_MATRIX
                    .queue_text(self.alarm_two.get_name(), 0, true, false)
                    .await
            }
        }
    }
}

impl App for AlarmApp {
    fn get_name(&self) -> &str {
        "Alarms"
    }

    async fn start(&mut self, _: Spawner) {
        self.show().await;
    }

    async fn stop(&mut self) {
        STOP_APP_CHANNEL
            .immediate_publisher()
            .publish_immediate(StopAppTasks);
    }

    async fn button_one_short_press(&mut self, spawner: Spawner) {
        if self.in_menu {
            self.in_menu = false;

            spawner.spawn(blink()).unwrap();

            match self.active_alarm {
                AlarmNumber::One => self.alarm_one.start(spawner).await,
                AlarmNumber::Two => self.alarm_two.start(spawner).await,
            }
        } else {
            match self.active_alarm {
                AlarmNumber::One => self.alarm_one.button_one_short_press(spawner).await,
                AlarmNumber::Two => self.alarm_two.button_one_short_press(spawner).await,
            }
        }
    }

    async fn button_two_press(&mut self, press: ButtonPress, spawner: Spawner) {
        if self.in_menu {
            match self.active_alarm {
                AlarmNumber::One => self.active_alarm = AlarmNumber::Two,
                AlarmNumber::Two => self.active_alarm = AlarmNumber::One,
            }

            self.show().await;
        } else {
            match self.active_alarm {
                AlarmNumber::One => self.alarm_one.button_two_press(press, spawner).await,
                AlarmNumber::Two => self.alarm_two.button_two_press(press, spawner).await,
            }
        }
    }

    async fn button_three_press(&mut self, press: ButtonPress, spawner: Spawner) {
        if self.in_menu {
            match self.active_alarm {
                AlarmNumber::One => self.active_alarm = AlarmNumber::Two,
                AlarmNumber::Two => self.active_alarm = AlarmNumber::One,
            }

            self.show().await;
        } else {
            match self.active_alarm {
                AlarmNumber::One => self.alarm_one.button_three_press(press, spawner).await,
                AlarmNumber::Two => self.alarm_two.button_three_press(press, spawner).await,
            }
        }
    }
}

/// Each of the possible configurations, but with data so the blink task can be displayed accurately.
enum BlinkTask {
    /// Use to keep the blink task going but not set the display.
    None,

    /// Blink the hour section of the display. (hour, minute)
    Hour(u32, u32),

    /// Blink the minute section of the display. (hour, minute)
    Minute(u32, u32),

    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

/// Named struct for next settings start signal.
struct NextAlarmPartStart;

/// Channel for firing events of when tasks should be stopped.
static STOP_APP_CHANNEL: PubSubChannel<ThreadModeRawMutex, StopAppTasks, 1, 1, 1> =
    PubSubChannel::new();

/// Signal for when the next item in settings is being configured.
static NEXT_ALARM_PART_START: Signal<ThreadModeRawMutex, NextAlarmPartStart> = Signal::new();

/// Signal for blink task to know what the item that should be blinked.
static ALARM_DISPLAY_QUEUE: Signal<ThreadModeRawMutex, BlinkTask> = Signal::new();

/// Blink the active configuration background task.
#[embassy_executor::task]
async fn blink() {
    let mut stop_task_sub = STOP_APP_CHANNEL.subscriber().unwrap();
    let mut blink_task = BlinkTask::Hour(0, 0);

    let mut blink_iteration = false;
    let mut wait_delay;

    loop {
        if ALARM_DISPLAY_QUEUE.signaled() {
            blink_task = ALARM_DISPLAY_QUEUE.wait().await;
        }

        if blink_iteration {
            wait_delay = 350;
        } else {
            wait_delay = 750;
        }

        match blink_task {
            BlinkTask::None => {}
            BlinkTask::Hour(hour, min) => {
                if blink_iteration {
                    DISPLAY_MATRIX
                        .queue_time_left_side_blink(min, wait_delay, false)
                        .await;
                } else {
                    DISPLAY_MATRIX
                        .queue_time(hour, min, TimeColon::Full, wait_delay, true, false)
                        .await;
                }
            }
            BlinkTask::Minute(hour, min) => {
                if blink_iteration {
                    DISPLAY_MATRIX
                        .queue_time_right_side_blink(min, wait_delay, false)
                        .await;
                } else {
                    DISPLAY_MATRIX
                        .queue_time(hour, min, TimeColon::Full, wait_delay, true, false)
                        .await;
                }
            }
            BlinkTask::Monday => {
                if blink_iteration {
                    DISPLAY_MATRIX.hide_icon("Mon");
                } else {
                    DISPLAY_MATRIX.show_icon("Mon");
                }
            }
            BlinkTask::Tuesday => {
                if blink_iteration {
                    DISPLAY_MATRIX.hide_icon("Tue");
                } else {
                    DISPLAY_MATRIX.show_icon("Tue");
                }
            }
            BlinkTask::Wednesday => {
                if blink_iteration {
                    DISPLAY_MATRIX.hide_icon("Wed");
                } else {
                    DISPLAY_MATRIX.show_icon("Wed");
                }
            }
            BlinkTask::Thursday => {
                if blink_iteration {
                    DISPLAY_MATRIX.hide_icon("Thur");
                } else {
                    DISPLAY_MATRIX.show_icon("Thur");
                }
            }
            BlinkTask::Friday => {
                if blink_iteration {
                    DISPLAY_MATRIX.hide_icon("Fri");
                } else {
                    DISPLAY_MATRIX.show_icon("Fri");
                }
            }
            BlinkTask::Saturday => {
                if blink_iteration {
                    DISPLAY_MATRIX.hide_icon("Sat");
                } else {
                    DISPLAY_MATRIX.show_icon("Sat");
                }
            }
            BlinkTask::Sunday => {
                if blink_iteration {
                    DISPLAY_MATRIX.hide_icon("Sun");
                } else {
                    DISPLAY_MATRIX.show_icon("Sun");
                }
            }
        }

        let wait_task = select3(
            stop_task_sub.next_message(),
            NEXT_ALARM_PART_START.wait(),
            Timer::after(Duration::from_millis(wait_delay)),
        )
        .await;

        match wait_task {
            First(_) => break,
            Second(_) => {}
            Third(_) => {}
        }

        if blink_iteration {
            blink_iteration = false;
        } else {
            blink_iteration = true;
        }
    }
}

/// All alarm configurations mini apps.
mod configurations {
    use embassy_executor::Spawner;

    use crate::{app::App, buttons::ButtonPress, display::display_matrix::DISPLAY_MATRIX};

    use super::{AlarmNumber, ALARM_DISPLAY_QUEUE};

    /// Common trait that all settings configs should implement.
    pub trait Configuration {
        /// Start the configuration.
        async fn start(&mut self);

        /// Save and stop the configuration.
        async fn save(&mut self);

        /// Handle middle button press.
        async fn button_two_press(&mut self, press: ButtonPress);

        /// Handle bottom button press.
        async fn button_three_press(&mut self, press: ButtonPress);
    }

    trait Saveable {
        fn should_save(&self) -> bool;
    }

    trait ButtonModify {
        async fn button_two_press(&mut self, press: ButtonPress);

        async fn button_three_press(&mut self, press: ButtonPress);
    }

    struct AlarmHour {
        /// The hour being configured.
        hour: u32,

        /// The hour set when starting configuration.
        starting_hour: u32,
    }

    impl Saveable for AlarmHour {
        fn should_save(&self) -> bool {
            self.hour != self.starting_hour
        }
    }

    impl ButtonModify for AlarmHour {
        async fn button_two_press(&mut self, _: ButtonPress) {
            if self.hour == 23 {
                self.hour = 0;
            } else {
                self.hour += 1;
            }
        }

        async fn button_three_press(&mut self, _: ButtonPress) {
            if self.hour == 0 {
                self.hour = 23;
            } else {
                self.hour -= 1;
            }
        }
    }

    struct AlarmMinute {
        /// The minute being configured.
        minute: u32,

        /// The minute set when starting configuration.
        starting_minute: u32,
    }

    impl Saveable for AlarmMinute {
        fn should_save(&self) -> bool {
            self.minute != self.starting_minute
        }
    }

    impl ButtonModify for AlarmMinute {
        async fn button_two_press(&mut self, _: ButtonPress) {
            if self.minute == 59 {
                self.minute = 0;
            } else {
                self.minute += 1;
            }
        }

        async fn button_three_press(&mut self, _: ButtonPress) {
            if self.minute == 0 {
                self.minute = 59;
            } else {
                self.minute -= 1;
            }
        }
    }

    struct AlarmDay {
        state: bool,
        starting_state: bool,
    }

    impl AlarmDay {
        async fn display_state(&self) {
            if self.state {
                DISPLAY_MATRIX.queue_text("ON", 0, true, false).await;
            } else {
                DISPLAY_MATRIX.queue_text("OFF", 0, true, false).await;
            }
        }
    }

    impl Saveable for AlarmDay {
        fn should_save(&self) -> bool {
            self.state != self.starting_state
        }
    }

    impl ButtonModify for AlarmDay {
        async fn button_two_press(&mut self, _: ButtonPress) {
            self.state = !self.state;
        }

        async fn button_three_press(&mut self, _: ButtonPress) {
            self.state = !self.state;
        }
    }

    enum ActiveConfig {
        Hour,

        Minute,

        Monday,

        Tuesday,

        Wednesday,

        Thursday,

        Friday,

        Saturday,

        Sunday,
    }

    /// RTC hour configuration.
    pub struct AlarmConfiguration {
        alarm_number: AlarmNumber,

        hour: AlarmHour,

        minute: AlarmMinute,

        monday: AlarmDay,
        tuesday: AlarmDay,
        wednesday: AlarmDay,
        thursday: AlarmDay,
        friday: AlarmDay,
        saturday: AlarmDay,
        sunday: AlarmDay,

        active_config: ActiveConfig,
    }

    impl AlarmConfiguration {
        /// Create a new hour configuration.
        pub fn new(num: AlarmNumber) -> Self {
            Self {
                alarm_number: num,
                hour: AlarmHour {
                    hour: 0,
                    starting_hour: 0,
                },
                minute: AlarmMinute {
                    minute: 0,
                    starting_minute: 0,
                },
                monday: AlarmDay {
                    state: false,
                    starting_state: false,
                },
                tuesday: AlarmDay {
                    state: false,
                    starting_state: false,
                },
                wednesday: AlarmDay {
                    state: false,
                    starting_state: false,
                },
                thursday: AlarmDay {
                    state: false,
                    starting_state: false,
                },
                friday: AlarmDay {
                    state: false,
                    starting_state: false,
                },
                saturday: AlarmDay {
                    state: false,
                    starting_state: false,
                },
                sunday: AlarmDay {
                    state: false,
                    starting_state: false,
                },
                active_config: ActiveConfig::Hour,
            }
        }

        fn show_day_icons(&self) {
            if self.monday.state {
                DISPLAY_MATRIX.show_icon("Mon");
            } else {
                DISPLAY_MATRIX.hide_icon("Mon");
            }

            if self.tuesday.state {
                DISPLAY_MATRIX.show_icon("Tue");
            } else {
                DISPLAY_MATRIX.hide_icon("Tue");
            }

            if self.wednesday.state {
                DISPLAY_MATRIX.show_icon("Wed");
            } else {
                DISPLAY_MATRIX.hide_icon("Wed");
            }

            if self.thursday.state {
                DISPLAY_MATRIX.show_icon("Thur");
            } else {
                DISPLAY_MATRIX.hide_icon("Thur");
            }

            if self.friday.state {
                DISPLAY_MATRIX.show_icon("Fri");
            } else {
                DISPLAY_MATRIX.hide_icon("Fri");
            }

            if self.saturday.state {
                DISPLAY_MATRIX.show_icon("Sat");
            } else {
                DISPLAY_MATRIX.hide_icon("Sat");
            }

            if self.sunday.state {
                DISPLAY_MATRIX.show_icon("Sun");
            } else {
                DISPLAY_MATRIX.hide_icon("Sun");
            }
        }

        async fn show(&self) {
            match self.active_config {
                ActiveConfig::Hour => {
                    ALARM_DISPLAY_QUEUE
                        .signal(super::BlinkTask::Hour(self.hour.hour, self.minute.minute));
                }
                ActiveConfig::Minute => {
                    ALARM_DISPLAY_QUEUE
                        .signal(super::BlinkTask::Minute(self.hour.hour, self.minute.minute));
                }
                ActiveConfig::Monday => {
                    ALARM_DISPLAY_QUEUE.signal(super::BlinkTask::Monday);
                    self.show_day_icons();
                    self.monday.display_state().await;
                }
                ActiveConfig::Tuesday => {
                    ALARM_DISPLAY_QUEUE.signal(super::BlinkTask::Tuesday);
                    self.show_day_icons();
                    self.tuesday.display_state().await;
                }
                ActiveConfig::Wednesday => {
                    ALARM_DISPLAY_QUEUE.signal(super::BlinkTask::Wednesday);
                    self.show_day_icons();
                    self.wednesday.display_state().await;
                }
                ActiveConfig::Thursday => {
                    ALARM_DISPLAY_QUEUE.signal(super::BlinkTask::Thursday);
                    self.show_day_icons();
                    self.thursday.display_state().await;
                }
                ActiveConfig::Friday => {
                    ALARM_DISPLAY_QUEUE.signal(super::BlinkTask::Friday);
                    self.show_day_icons();
                    self.friday.display_state().await;
                }
                ActiveConfig::Saturday => {
                    ALARM_DISPLAY_QUEUE.signal(super::BlinkTask::Saturday);
                    self.show_day_icons();
                    self.saturday.display_state().await;
                }
                ActiveConfig::Sunday => {
                    ALARM_DISPLAY_QUEUE.signal(super::BlinkTask::Sunday);
                    self.show_day_icons();
                    self.sunday.display_state().await;
                }
            }
        }
    }

    impl App for AlarmConfiguration {
        fn get_name(&self) -> &str {
            match self.alarm_number {
                AlarmNumber::One => "One",
                AlarmNumber::Two => "Two",
            }
        }

        async fn start(&mut self, _: Spawner) {
            self.show().await;
        }

        async fn stop(&mut self) {}

        async fn button_one_short_press(&mut self, _: Spawner) {
            match self.active_config {
                ActiveConfig::Hour => {
                    self.active_config = ActiveConfig::Minute;
                }
                ActiveConfig::Minute => {
                    self.active_config = ActiveConfig::Monday;
                }
                ActiveConfig::Monday => {
                    self.active_config = ActiveConfig::Tuesday;
                }
                ActiveConfig::Tuesday => {
                    self.active_config = ActiveConfig::Wednesday;
                }
                ActiveConfig::Wednesday => {
                    self.active_config = ActiveConfig::Thursday;
                }
                ActiveConfig::Thursday => {
                    self.active_config = ActiveConfig::Friday;
                }
                ActiveConfig::Friday => {
                    self.active_config = ActiveConfig::Saturday;
                }
                ActiveConfig::Saturday => {
                    self.active_config = ActiveConfig::Sunday;
                }
                ActiveConfig::Sunday => {
                    // don't want to call normal show here, so call the important methods and return early
                    ALARM_DISPLAY_QUEUE.signal(super::BlinkTask::None);
                    self.show_day_icons();
                    DISPLAY_MATRIX.queue_text("Done", 2000, true, false).await;
                    return;
                }
            }

            self.show().await;
        }

        async fn button_two_press(&mut self, press: ButtonPress, _: Spawner) {
            match self.active_config {
                ActiveConfig::Hour => self.hour.button_two_press(press).await,
                ActiveConfig::Minute => self.minute.button_two_press(press).await,
                ActiveConfig::Monday => self.monday.button_two_press(press).await,
                ActiveConfig::Tuesday => self.tuesday.button_two_press(press).await,
                ActiveConfig::Wednesday => self.wednesday.button_two_press(press).await,
                ActiveConfig::Thursday => self.thursday.button_two_press(press).await,
                ActiveConfig::Friday => self.friday.button_two_press(press).await,
                ActiveConfig::Saturday => self.saturday.button_two_press(press).await,
                ActiveConfig::Sunday => self.sunday.button_two_press(press).await,
            }
            self.show().await;
        }

        async fn button_three_press(&mut self, press: ButtonPress, _: Spawner) {
            match self.active_config {
                ActiveConfig::Hour => self.hour.button_three_press(press).await,
                ActiveConfig::Minute => self.minute.button_three_press(press).await,
                ActiveConfig::Monday => self.monday.button_three_press(press).await,
                ActiveConfig::Tuesday => self.tuesday.button_three_press(press).await,
                ActiveConfig::Wednesday => self.wednesday.button_three_press(press).await,
                ActiveConfig::Thursday => self.thursday.button_three_press(press).await,
                ActiveConfig::Friday => self.friday.button_three_press(press).await,
                ActiveConfig::Saturday => self.saturday.button_three_press(press).await,
                ActiveConfig::Sunday => self.sunday.button_three_press(press).await,
            }
            self.show().await;
        }
    }
}
