use embassy_executor::Spawner;
use embassy_futures::select::{select3, Either3::*};
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex, pubsub::PubSubChannel, signal::Signal,
};
use embassy_time::{Duration, Timer};

use crate::{
    app::{App, ShowAppSwitcher, StopAppTasks, SHOW_APP_SWITCHER},
    buttons::ButtonPress,
    display::display_matrix::DISPLAY_MATRIX,
};

use self::configurations::{
    Configuration, DayConfiguration, HourConfiguration, MinuteConfiguration, MonthConfiguration,
    YearConfiguration,
};

/// Each of the possible configurations to run through in the settings app.
enum SettingsConfig {
    /// Modify the hour in the RTC.
    Hour,

    /// Modify the minute in the RTC.
    Minute,

    /// Modify the year in the RTC.
    Year,

    /// Modify the month in the RTC.
    Month,

    /// Modify the day in the RTC.
    Day,
}

/// Each of the possible configurations, but with data so the blink task can be displayed accurately.
enum BlinkTask {
    /// Blink the hour section of the display. (hour, minute)
    Hour(u32, u32),

    /// Blink the minute section of the display. (hour, minute)
    Minute(u32, u32),

    /// Blink the full year in the display.
    Year(i32),

    /// Blink the month section of the display. (month, day)
    Month(u32, u32),

    /// Blink the day section of the display. (month, day)
    Day(u32, u32),
}

/// Named struct for next settings start signal.
struct NextSettingsStart;

/// Channel for firing events of when tasks should be stopped.
static STOP_APP_CHANNEL: PubSubChannel<ThreadModeRawMutex, StopAppTasks, 1, 1, 1> =
    PubSubChannel::new();

/// Signal for when the next item in settings is being configured.
static NEXT_SETTINGS_START: Signal<ThreadModeRawMutex, NextSettingsStart> = Signal::new();

/// Signal for blink task to know what the item that should be blinked.
static SETTINGS_DISPLAY_QUEUE: Signal<ThreadModeRawMutex, BlinkTask> = Signal::new();

/// Settings app.
/// Allows for setting RTC and will be expanded for more options.
pub struct SettingsApp {
    /// The hour configuration mini app.
    hour_config: configurations::HourConfiguration,

    /// The minute configuration mini app.
    minute_config: configurations::MinuteConfiguration,

    /// The year configuration mini app.
    year_config: configurations::YearConfiguration,

    /// The month configuration mini app.
    month_config: configurations::MonthConfiguration,

    /// The day configuration mini app.
    day_config: configurations::DayConfiguration,

    /// The current active mini app being configured.
    active_config: SettingsConfig,
}

impl SettingsApp {
    /// Create a new settings app.
    pub fn new() -> Self {
        Self {
            hour_config: HourConfiguration::new(),
            minute_config: MinuteConfiguration::new(),
            year_config: YearConfiguration::new(),
            month_config: MonthConfiguration::new(),
            day_config: DayConfiguration::new(),
            active_config: SettingsConfig::Hour,
        }
    }
}

impl App for SettingsApp {
    fn get_name(&self) -> &str {
        "Settings"
    }

    async fn start(&mut self, spawner: Spawner) {
        critical_section::with(|cs| {
            DISPLAY_MATRIX.clear_all(cs, true);
        });

        self.active_config = SettingsConfig::Hour;
        self.hour_config.start().await;

        spawner.spawn(blink()).unwrap();
    }

    async fn stop(&mut self) {
        STOP_APP_CHANNEL
            .immediate_publisher()
            .publish_immediate(StopAppTasks);
    }

    async fn button_one_short_press(&mut self, _: Spawner) {
        match self.active_config {
            SettingsConfig::Hour => {
                self.hour_config.save().await;
                self.active_config = SettingsConfig::Minute;
                self.minute_config.start().await;
            }
            SettingsConfig::Minute => {
                self.minute_config.save().await;
                self.active_config = SettingsConfig::Year;
                self.year_config.start().await;
            }
            SettingsConfig::Year => {
                self.year_config.save().await;
                self.active_config = SettingsConfig::Month;
                self.month_config.start().await;
            }
            SettingsConfig::Month => {
                self.month_config.save().await;
                self.active_config = SettingsConfig::Day;
                self.day_config.start().await;
            }
            SettingsConfig::Day => {
                self.day_config.save().await;
                self.end().await;
            }
        }

        NEXT_SETTINGS_START.signal(NextSettingsStart);
    }

    async fn button_two_press(&mut self, press: ButtonPress, _: Spawner) {
        match self.active_config {
            SettingsConfig::Hour => self.hour_config.button_two_press(press).await,
            SettingsConfig::Minute => self.minute_config.button_two_press(press).await,
            SettingsConfig::Year => self.year_config.button_two_press(press).await,
            SettingsConfig::Month => self.month_config.button_two_press(press).await,
            SettingsConfig::Day => self.day_config.button_two_press(press).await,
        }
    }

    async fn button_three_press(&mut self, press: ButtonPress, _: Spawner) {
        match self.active_config {
            SettingsConfig::Hour => self.hour_config.button_three_press(press).await,
            SettingsConfig::Minute => self.minute_config.button_three_press(press).await,
            SettingsConfig::Year => self.year_config.button_three_press(press).await,
            SettingsConfig::Month => self.month_config.button_three_press(press).await,
            SettingsConfig::Day => self.day_config.button_three_press(press).await,
        }
    }
}

impl SettingsApp {
    /// End of settings configuration.
    ///
    /// Stop tasks, show "Done" and then show app switcher after delay.
    async fn end(&mut self) {
        self.stop().await;
        DISPLAY_MATRIX.queue_text("Done", 2000, true).await;
        Timer::after(Duration::from_secs(2)).await;
        SHOW_APP_SWITCHER.signal(ShowAppSwitcher);
    }
}

/// Blink the active configuration background task.
#[embassy_executor::task]
async fn blink() {
    let mut stop_task_sub = STOP_APP_CHANNEL.subscriber().unwrap();
    let mut blink_task = BlinkTask::Hour(0, 0);

    loop {
        if SETTINGS_DISPLAY_QUEUE.signaled() {
            blink_task = SETTINGS_DISPLAY_QUEUE.wait().await;
        }

        match blink_task {
            BlinkTask::Hour(hour, min) => {
                DISPLAY_MATRIX.queue_time(hour, min, 750, true).await;
                DISPLAY_MATRIX
                    .queue_time_left_side_blink(min, 350, false)
                    .await;
            }
            BlinkTask::Minute(hour, min) => {
                DISPLAY_MATRIX.queue_time(hour, min, 750, true).await;
                DISPLAY_MATRIX
                    .queue_time_right_side_blink(hour, 350, false)
                    .await;
            }
            BlinkTask::Year(year) => {
                DISPLAY_MATRIX.queue_year(year, 750, true).await;
                DISPLAY_MATRIX.queue_text(" ", 350, false).await;
            }
            BlinkTask::Month(month, day) => {
                DISPLAY_MATRIX.queue_date(month, day, 750, true).await;
                DISPLAY_MATRIX
                    .queue_date_left_side_blink(day, 350, false)
                    .await;
            }
            BlinkTask::Day(month, day) => {
                DISPLAY_MATRIX.queue_date(month, day, 750, true).await;
                DISPLAY_MATRIX
                    .queue_date_right_side_blink(month, 350, false)
                    .await;
            }
        }

        let wait_task = select3(
            stop_task_sub.next_message(),
            NEXT_SETTINGS_START.wait(),
            Timer::after(Duration::from_millis(1100)),
        )
        .await;

        match wait_task {
            First(_) => break,
            Second(_) => {}
            Third(_) => {}
        }
    }
}

/// All settings configurations mini apps.
mod configurations {
    use crate::{buttons::ButtonPress, rtc};

    use super::SETTINGS_DISPLAY_QUEUE;

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

    /// RTC hour configuration.
    pub struct HourConfiguration {
        hour: u32,
    }

    impl Configuration for HourConfiguration {
        async fn start(&mut self) {
            self.hour = rtc::get_hour().await;
            self.show().await;
        }

        async fn save(&mut self) {
            rtc::set_hour(self.hour).await;
        }

        async fn button_two_press(&mut self, _: ButtonPress) {
            if self.hour == 23 {
                self.hour = 0;
            } else {
                self.hour += 1;
            }
            self.show().await;
        }

        async fn button_three_press(&mut self, _: ButtonPress) {
            if self.hour == 0 {
                self.hour = 23;
            } else {
                self.hour -= 1;
            }
            self.show().await;
        }
    }

    impl HourConfiguration {
        /// Create a new hour configuration.
        pub fn new() -> Self {
            Self { hour: 0 }
        }

        /// Show hour configuration in blink task.
        async fn show(&self) {
            let minute = rtc::get_minute().await;
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::Hour(self.hour, minute));
        }
    }

    /// RTC minute configuration.
    pub struct MinuteConfiguration {
        minute: u32,
    }

    impl Configuration for MinuteConfiguration {
        async fn start(&mut self) {
            self.minute = rtc::get_minute().await;
            self.show().await;
        }

        async fn save(&mut self) {
            rtc::set_minute(self.minute).await;
        }

        async fn button_two_press(&mut self, _: ButtonPress) {
            if self.minute == 59 {
                self.minute = 0;
            } else {
                self.minute += 1;
            }
            self.show().await;
        }

        async fn button_three_press(&mut self, _: ButtonPress) {
            if self.minute == 0 {
                self.minute = 59;
            } else {
                self.minute -= 1;
            }
            self.show().await;
        }
    }

    impl MinuteConfiguration {
        /// Create a new minute configuration.
        pub fn new() -> Self {
            Self { minute: 0 }
        }

        /// Show minute configuration in blink task.
        async fn show(&self) {
            let hour = rtc::get_hour().await;
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::Minute(hour, self.minute));
        }
    }

    /// RTC year configuration.
    pub struct YearConfiguration {
        year: i32,
    }

    impl Configuration for YearConfiguration {
        async fn start(&mut self) {
            self.year = rtc::get_year().await;
            self.show().await;
        }

        async fn save(&mut self) {
            rtc::set_year(self.year).await;
        }

        async fn button_two_press(&mut self, _: ButtonPress) {
            if self.year == 2100 {
                self.year = 2000;
            } else {
                self.year += 1;
            }
            self.show().await;
        }

        async fn button_three_press(&mut self, _: ButtonPress) {
            if self.year == 2000 {
                self.year = 2100;
            } else {
                self.year -= 1;
            }
            self.show().await;
        }
    }

    impl YearConfiguration {
        /// Create a new year configuration.
        pub fn new() -> Self {
            Self { year: 0 }
        }

        /// Show year configuration in blink task.
        async fn show(&self) {
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::Year(self.year));
        }
    }

    /// RTC month configuration.
    pub struct MonthConfiguration {
        month: u32,
    }

    impl Configuration for MonthConfiguration {
        async fn start(&mut self) {
            self.month = rtc::get_month().await;
            self.show().await;
        }

        async fn save(&mut self) {
            rtc::set_month(self.month).await;
        }

        async fn button_two_press(&mut self, _: ButtonPress) {
            if self.month == 12 {
                self.month = 1;
            } else {
                self.month += 1;
            }
            self.show().await;
        }

        async fn button_three_press(&mut self, _: ButtonPress) {
            if self.month == 1 {
                self.month = 12;
            } else {
                self.month -= 1;
            }
            self.show().await;
        }
    }

    impl MonthConfiguration {
        /// Create a new month configuration.
        pub fn new() -> Self {
            Self { month: 0 }
        }

        /// Show minute configuration in blink task.
        async fn show(&self) {
            let day = rtc::get_day().await;
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::Month(self.month, day));
        }
    }

    /// RTC day configuration.
    pub struct DayConfiguration {
        day: u32,
        month: u32,
    }

    impl Configuration for DayConfiguration {
        async fn start(&mut self) {
            self.day = rtc::get_day().await;
            self.month = rtc::get_month().await;
            self.show().await;
        }

        async fn save(&mut self) {
            rtc::set_day(self.day).await;
        }

        async fn button_two_press(&mut self, _: ButtonPress) {
            if self.day == rtc::get_max_day_in_month(self.month).await {
                self.day = 1;
            } else {
                self.day += 1;
            }
            self.show().await;
        }

        async fn button_three_press(&mut self, _: ButtonPress) {
            if self.day == 1 {
                self.day = rtc::get_max_day_in_month(self.month).await;
            } else {
                self.day -= 1;
            }
            self.show().await;
        }
    }

    impl DayConfiguration {
        /// Create a new day configuration.
        pub fn new() -> Self {
            Self { day: 0, month: 0 }
        }

        /// Show day configuration in blink task.
        async fn show(&self) {
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::Day(self.month, self.day));
        }
    }
}
