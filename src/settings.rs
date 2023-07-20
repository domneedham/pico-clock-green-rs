use embassy_executor::Spawner;

use crate::{app::App, buttons::ButtonPress, display::display_matrix::DISPLAY_MATRIX};

use self::configurations::{
    Configuration, DayConfiguration, HourConfiguration, MinuteConfiguration,
};

enum SettingsConfig {
    Hour,
    Minute,
    Day,
    Month,
    Year,
}

pub struct SettingsApp<'a> {
    name: &'a str,
    hour_config: configurations::HourConfiguration,
    minute_config: configurations::MinuteConfiguration,
    day_config: configurations::DayConfiguration,
    active_config: SettingsConfig,
}

impl<'a> SettingsApp<'a> {
    pub fn new(name: &'a str) -> Self {
        Self {
            name,
            hour_config: HourConfiguration::new(),
            minute_config: MinuteConfiguration::new(),
            day_config: DayConfiguration::new(),
            active_config: SettingsConfig::Hour,
        }
    }
}

impl<'a> App<'a> for SettingsApp<'a> {
    fn get_name(&self) -> &'a str {
        self.name
    }

    async fn start(&mut self, _: Spawner) {
        critical_section::with(|cs| {
            DISPLAY_MATRIX.clear_all(cs, true);
        });

        self.active_config = SettingsConfig::Hour;
        self.hour_config.start().await;
    }

    async fn stop(&mut self) {}

    async fn button_one_short_press(&mut self, _: Spawner) {
        match self.active_config {
            SettingsConfig::Hour => {
                self.hour_config.save().await;
                self.active_config = SettingsConfig::Minute;
                self.minute_config.start().await;
            }
            SettingsConfig::Minute => {
                self.minute_config.save().await;
                self.active_config = SettingsConfig::Day;
                self.day_config.start().await;
            }
            SettingsConfig::Day => {
                self.day_config.save().await;
                self.active_config = SettingsConfig::Month;
            }
            SettingsConfig::Month => todo!(),
            SettingsConfig::Year => todo!(),
        }
    }

    async fn button_two_press(&mut self, press: ButtonPress, _: Spawner) {
        match self.active_config {
            SettingsConfig::Hour => self.hour_config.button_two_press(press).await,
            SettingsConfig::Minute => self.minute_config.button_two_press(press).await,
            SettingsConfig::Day => self.day_config.button_two_press(press).await,
            SettingsConfig::Month => todo!(),
            SettingsConfig::Year => todo!(),
        }
    }

    async fn button_three_press(&mut self, press: ButtonPress, _: Spawner) {
        match self.active_config {
            SettingsConfig::Hour => self.hour_config.button_three_press(press).await,
            SettingsConfig::Minute => self.minute_config.button_three_press(press).await,
            SettingsConfig::Day => self.day_config.button_three_press(press).await,
            SettingsConfig::Month => todo!(),
            SettingsConfig::Year => todo!(),
        }
    }
}

mod configurations {
    use crate::{buttons::ButtonPress, display::display_matrix::DISPLAY_MATRIX, rtc};

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
            DISPLAY_MATRIX.queue_time(self.hour, minute, true).await;
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
            DISPLAY_MATRIX.queue_time(hour, self.minute, true).await;
        }
    }

    pub struct DayConfiguration {
        day: u32,
    }

    impl Configuration for DayConfiguration {
        async fn start(&mut self) {
            self.day = rtc::get_day().await;
            self.show().await;
        }

        async fn save(&mut self) {
            rtc::set_day(self.day).await;
        }

        async fn button_two_press(&mut self, _: ButtonPress) {
            if self.day == 59 {
                self.day = 0;
            } else {
                self.day += 1;
            }
            self.show().await;
        }

        async fn button_three_press(&mut self, _: ButtonPress) {
            if self.day == 0 {
                self.day = 59;
            } else {
                self.day -= 1;
            }
            self.show().await;
        }
    }

    impl DayConfiguration {
        pub fn new() -> Self {
            Self { day: 0 }
        }

        async fn show(&self) {
            let month = rtc::get_month().await;
            DISPLAY_MATRIX.queue_date(self.day, month, true).await;
        }
    }
}
