use embassy_executor::Spawner;
use embassy_futures::select::{select3, Either3::*};
use embassy_sync::{
    blocking_mutex::raw::ThreadModeRawMutex, pubsub::PubSubChannel, signal::Signal,
};
use embassy_time::{Duration, Timer};

use crate::{
    app::{App, ShowAppSwitcher, StopAppTasks, SHOW_APP_SWITCHER},
    buttons::ButtonPress,
    display::display_matrix::{TimeColon, DISPLAY_MATRIX},
};

use self::configurations::{
    AutoScrollTempConfiguration, Configuration, DayConfiguration, HourConfiguration,
    HourlyRingConfiguration, MinuteConfiguration, MonthConfiguration, TimeColonConfiguration,
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

    /// Modify the hourly ring setting.
    HourlyRing,

    /// Modify the time colon setting.
    TimeColon,

    /// Modify the auto scrolling of temperature setting.
    AutoScrollTemp,
}

/// Each of the possible configurations, but with data so the blink task can be displayed accurately.
enum BlinkTask {
    /// Use to keep the blink task going but not set the display.
    None,

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

    /// The hourly ring configuration mini app.
    hourly_ring_config: configurations::HourlyRingConfiguration,

    /// The time colon configuration mini app.
    time_colon_config: configurations::TimeColonConfiguration,

    /// The auto scroll temp configuration mini app.
    auto_scroll_temp_config: configurations::AutoScrollTempConfiguration,

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
            hourly_ring_config: HourlyRingConfiguration::new(),
            time_colon_config: TimeColonConfiguration::new(),
            auto_scroll_temp_config: AutoScrollTempConfiguration::new(),
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
                self.active_config = SettingsConfig::HourlyRing;
                self.hourly_ring_config.start().await;
            }
            SettingsConfig::HourlyRing => {
                self.hourly_ring_config.save().await;
                self.active_config = SettingsConfig::TimeColon;
                self.time_colon_config.start().await;
            }
            SettingsConfig::TimeColon => {
                self.time_colon_config.save().await;
                self.active_config = SettingsConfig::AutoScrollTemp;
                self.auto_scroll_temp_config.start().await;
            }
            SettingsConfig::AutoScrollTemp => {
                self.auto_scroll_temp_config.save().await;
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
            SettingsConfig::HourlyRing => self.hourly_ring_config.button_two_press(press).await,
            SettingsConfig::TimeColon => self.time_colon_config.button_two_press(press).await,
            SettingsConfig::AutoScrollTemp => {
                self.auto_scroll_temp_config.button_two_press(press).await
            }
        }
    }

    async fn button_three_press(&mut self, press: ButtonPress, _: Spawner) {
        match self.active_config {
            SettingsConfig::Hour => self.hour_config.button_three_press(press).await,
            SettingsConfig::Minute => self.minute_config.button_three_press(press).await,
            SettingsConfig::Year => self.year_config.button_three_press(press).await,
            SettingsConfig::Month => self.month_config.button_three_press(press).await,
            SettingsConfig::Day => self.day_config.button_three_press(press).await,
            SettingsConfig::HourlyRing => self.hourly_ring_config.button_two_press(press).await,
            SettingsConfig::TimeColon => self.time_colon_config.button_three_press(press).await,
            SettingsConfig::AutoScrollTemp => {
                self.auto_scroll_temp_config.button_three_press(press).await
            }
        }
    }
}

impl SettingsApp {
    /// End of settings configuration.
    ///
    /// Stop tasks, show "Done" and then show app switcher after delay.
    async fn end(&mut self) {
        self.stop().await;
        DISPLAY_MATRIX.queue_text("Done", 2000, true, false).await;
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
            BlinkTask::None => {}
            BlinkTask::Hour(hour, min) => {
                DISPLAY_MATRIX
                    .queue_time(hour, min, TimeColon::Full, 750, true, false)
                    .await;
                DISPLAY_MATRIX
                    .queue_time_left_side_blink(min, 350, false)
                    .await;
            }
            BlinkTask::Minute(hour, min) => {
                DISPLAY_MATRIX
                    .queue_time(hour, min, TimeColon::Full, 750, true, false)
                    .await;
                DISPLAY_MATRIX
                    .queue_time_right_side_blink(hour, 350, false)
                    .await;
            }
            BlinkTask::Year(year) => {
                DISPLAY_MATRIX.queue_year(year, 750, true).await;
                DISPLAY_MATRIX.queue_text(" ", 350, false, false).await;
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
    use core::fmt::Write;
    use heapless::String;

    use crate::{
        buttons::ButtonPress,
        config::{self, TimeColonPreference},
        display::display_matrix::DISPLAY_MATRIX,
        rtc,
    };

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
        /// The hour being configured.
        hour: u32,

        /// The hour set when starting configuration.
        starting_hour: u32,
    }

    impl Configuration for HourConfiguration {
        async fn start(&mut self) {
            self.hour = rtc::get_hour().await;
            self.starting_hour = self.hour;
            self.show().await;
        }

        async fn save(&mut self) {
            if self.hour != self.starting_hour {
                rtc::set_hour(self.hour).await;
            }
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
            Self {
                hour: 0,
                starting_hour: 0,
            }
        }

        /// Show hour configuration in blink task.
        async fn show(&self) {
            let minute = rtc::get_minute().await;
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::Hour(self.hour, minute));
        }
    }

    /// RTC minute configuration.
    pub struct MinuteConfiguration {
        /// The minute being configured.
        minute: u32,

        /// The minute set when starting configuration.
        starting_minute: u32,
    }

    impl Configuration for MinuteConfiguration {
        async fn start(&mut self) {
            self.minute = rtc::get_minute().await;
            self.starting_minute = self.minute;
            self.show().await;
        }

        async fn save(&mut self) {
            if self.minute != self.starting_minute {
                rtc::set_minute(self.minute).await;
            }
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
            Self {
                minute: 0,
                starting_minute: 0,
            }
        }

        /// Show minute configuration in blink task.
        async fn show(&self) {
            let hour = rtc::get_hour().await;
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::Minute(hour, self.minute));
        }
    }

    /// RTC year configuration.
    pub struct YearConfiguration {
        /// The year being configured.
        year: i32,

        /// The year set when starting configuration.
        starting_year: i32,
    }

    impl Configuration for YearConfiguration {
        async fn start(&mut self) {
            self.year = rtc::get_year().await;
            self.starting_year = self.year;
            self.show().await;
        }

        async fn save(&mut self) {
            if self.year != self.starting_year {
                rtc::set_year(self.year).await;
            }
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
            Self {
                year: 0,
                starting_year: 0,
            }
        }

        /// Show year configuration in blink task.
        async fn show(&self) {
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::Year(self.year));
        }
    }

    /// RTC month configuration.
    pub struct MonthConfiguration {
        /// The month being configured.
        month: u32,

        /// The month set when starting configuration.
        starting_month: u32,
    }

    impl Configuration for MonthConfiguration {
        async fn start(&mut self) {
            self.month = rtc::get_month().await;
            self.starting_month = self.month;
            self.show().await;
        }

        async fn save(&mut self) {
            if self.month != self.starting_month {
                rtc::set_month(self.month).await;
            }
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
            Self {
                month: 0,
                starting_month: 0,
            }
        }

        /// Show minute configuration in blink task.
        async fn show(&self) {
            let day = rtc::get_day().await;
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::Month(self.month, day));
        }
    }

    /// RTC day configuration.
    pub struct DayConfiguration {
        /// The day being configured.
        day: u32,

        /// The day set when starting configuration.
        starting_day: u32,

        /// The current month in RTC. This is purely just a reference and should not be mutated.
        month: u32,
    }

    impl Configuration for DayConfiguration {
        async fn start(&mut self) {
            self.day = rtc::get_day().await;
            self.starting_day = self.day;
            self.month = rtc::get_month().await;
            self.show().await;
        }

        async fn save(&mut self) {
            if self.day != self.starting_day {
                rtc::set_day(self.day).await;
            }
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
            Self {
                day: 0,
                starting_day: 0,
                month: 0,
            }
        }

        /// Show day configuration in blink task.
        async fn show(&self) {
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::Day(self.month, self.day));
        }
    }

    /// RTC day configuration.
    pub struct HourlyRingConfiguration {
        /// The ring state.
        state: bool,

        /// The state set when starting configuration.
        starting_state: bool,
    }

    impl Configuration for HourlyRingConfiguration {
        async fn start(&mut self) {
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::None);
            self.state = config::CONFIG
                .lock()
                .await
                .borrow()
                .as_ref()
                .unwrap()
                .get_hourly_ring();
            self.starting_state = self.state;
            self.show().await;
        }

        async fn save(&mut self) {
            if self.state != self.starting_state {
                config::CONFIG
                    .lock()
                    .await
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .set_hourly_ring(self.state);
            }
        }

        async fn button_two_press(&mut self, _: ButtonPress) {
            self.state = !self.state;
            self.show().await;
        }

        async fn button_three_press(&mut self, _: ButtonPress) {
            self.state = !self.state;
            self.show().await;
        }
    }

    impl HourlyRingConfiguration {
        /// Create a new day configuration.
        pub fn new() -> Self {
            Self {
                state: false,
                starting_state: false,
            }
        }

        /// Show day configuration in blink task.
        async fn show(&self) {
            let mut text: String<16> = String::new();
            _ = write!(text, "HR:");
            if self.state {
                _ = write!(text, "On");
            } else {
                _ = write!(text, "Of");
            }

            DISPLAY_MATRIX
                .queue_text(text.as_str(), 1000, true, false)
                .await;
        }
    }

    /// RTC day configuration.
    pub struct TimeColonConfiguration {
        /// The ring state.
        state: TimeColonPreference,

        /// The state set when starting configuration.
        starting_state: TimeColonPreference,
    }

    impl Configuration for TimeColonConfiguration {
        async fn start(&mut self) {
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::None);
            self.state = config::CONFIG
                .lock()
                .await
                .borrow()
                .as_ref()
                .unwrap()
                .get_time_colon_preference();
            self.starting_state = self.state;
            self.show().await;
        }

        async fn save(&mut self) {
            if self.state != self.starting_state {
                config::CONFIG
                    .lock()
                    .await
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .set_time_colon_preference(self.state);
            }
        }

        async fn button_two_press(&mut self, _: ButtonPress) {
            match self.state {
                TimeColonPreference::Solid => self.state = TimeColonPreference::Blink,
                TimeColonPreference::Blink => self.state = TimeColonPreference::Alt,
                TimeColonPreference::Alt => self.state = TimeColonPreference::Solid,
            }
            self.show().await;
        }

        async fn button_three_press(&mut self, _: ButtonPress) {
            match self.state {
                TimeColonPreference::Solid => self.state = TimeColonPreference::Alt,
                TimeColonPreference::Blink => self.state = TimeColonPreference::Solid,
                TimeColonPreference::Alt => self.state = TimeColonPreference::Blink,
            }
            self.show().await;
        }
    }

    impl TimeColonConfiguration {
        /// Create a new day configuration.
        pub fn new() -> Self {
            Self {
                state: TimeColonPreference::Blink,
                starting_state: TimeColonPreference::Blink,
            }
        }

        /// Show day configuration in blink task.
        async fn show(&self) {
            let text = match self.state {
                TimeColonPreference::Solid => ":SLD",
                TimeColonPreference::Blink => ":BLK",
                TimeColonPreference::Alt => ":ALT",
            };

            DISPLAY_MATRIX.queue_text(text, 1000, true, false).await;
        }
    }

    /// RTC day configuration.
    pub struct AutoScrollTempConfiguration {
        /// The ring state.
        state: bool,

        /// The state set when starting configuration.
        starting_state: bool,
    }

    impl Configuration for AutoScrollTempConfiguration {
        async fn start(&mut self) {
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::None);
            self.state = config::CONFIG
                .lock()
                .await
                .borrow()
                .as_ref()
                .unwrap()
                .get_auto_scroll_temp();
            self.starting_state = self.state;
            self.show().await;
        }

        async fn save(&mut self) {
            if self.state != self.starting_state {
                config::CONFIG
                    .lock()
                    .await
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .set_auto_scroll_temp(self.state);
            }
        }

        async fn button_two_press(&mut self, _: ButtonPress) {
            self.state = !self.state;
            self.show().await;
        }

        async fn button_three_press(&mut self, _: ButtonPress) {
            self.state = !self.state;
            self.show().await;
        }
    }

    impl AutoScrollTempConfiguration {
        /// Create a new day configuration.
        pub fn new() -> Self {
            Self {
                state: false,
                starting_state: false,
            }
        }

        /// Show day configuration in blink task.
        async fn show(&self) {
            let mut text: String<16> = String::new();
            _ = write!(text, "EX:");
            if self.state {
                _ = write!(text, "On");
            } else {
                _ = write!(text, "Of");
            }

            DISPLAY_MATRIX
                .queue_text(text.as_str(), 1000, true, false)
                .await;
        }
    }
}
