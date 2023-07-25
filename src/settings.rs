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

enum SettingsConfig {
    Hour,
    Minute,
    Year,
    Month,
    Day,
}

enum BlinkTask {
    Hour(u32, u32),
    Minute(u32, u32),
    Year(i32),
    Month(u32, u32),
    Day(u32, u32),
}

struct NextSettingsStart();

static STOP_APP_CHANNEL: PubSubChannel<ThreadModeRawMutex, StopAppTasks, 1, 1, 1> =
    PubSubChannel::new();

static NEXT_SETTINGS_START: Signal<ThreadModeRawMutex, NextSettingsStart> = Signal::new();
static SETTINGS_DISPLAY_QUEUE: Signal<ThreadModeRawMutex, BlinkTask> = Signal::new();

pub struct SettingsApp {
    hour_config: configurations::HourConfiguration,
    minute_config: configurations::MinuteConfiguration,
    year_config: configurations::YearConfiguration,
    month_config: configurations::MonthConfiguration,
    day_config: configurations::DayConfiguration,
    active_config: SettingsConfig,
}

impl SettingsApp {
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

        NEXT_SETTINGS_START.signal(NextSettingsStart());
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
    async fn end(&mut self) {
        self.stop().await;
        DISPLAY_MATRIX.queue_text("Done", 2000, true).await;
        Timer::after(Duration::from_secs(2)).await;
        SHOW_APP_SWITCHER.signal(ShowAppSwitcher);
    }
}

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

mod configurations {
    use crate::{buttons::ButtonPress, rtc};

    use super::SETTINGS_DISPLAY_QUEUE;

    pub trait Configuration {
        async fn start(&mut self);
        async fn save(&mut self);

        async fn button_two_press(&mut self, press: ButtonPress);
        async fn button_three_press(&mut self, press: ButtonPress);
    }

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
        pub fn new() -> Self {
            Self { hour: 0 }
        }

        async fn show(&self) {
            let minute = rtc::get_minute().await;
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::Hour(self.hour, minute));
        }
    }

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
        pub fn new() -> Self {
            Self { minute: 0 }
        }

        async fn show(&self) {
            let hour = rtc::get_hour().await;
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::Minute(hour, self.minute));
        }
    }

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
        pub fn new() -> Self {
            Self { year: 0 }
        }

        async fn show(&self) {
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::Year(self.year));
        }
    }

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
        pub fn new() -> Self {
            Self { month: 0 }
        }

        async fn show(&self) {
            let day = rtc::get_day().await;
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::Month(self.month, day));
        }
    }

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
        pub fn new() -> Self {
            Self { day: 0, month: 0 }
        }

        async fn show(&self) {
            SETTINGS_DISPLAY_QUEUE.signal(super::BlinkTask::Day(self.month, self.day));
        }
    }
}
