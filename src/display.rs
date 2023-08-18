use core::cell::RefCell;
use core::fmt::Write;
use critical_section::{CriticalSection, Mutex};
use defmt::info;
use embassy_rp::gpio::Output;
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, channel::Channel};
use embassy_time::{Duration, Timer};
use heapless::Vec;

use self::{
    icons::{get_icon_struct, Icon},
    text::{get_character_struct, Character},
};

/// All the pins required for the display.
pub struct DisplayPins<'a> {
    /// A0 pin.
    a0: Output<'a, embassy_rp::peripherals::PIN_16>,

    /// A1 pin.
    a1: Output<'a, embassy_rp::peripherals::PIN_18>,

    /// A2 pin.
    a2: Output<'a, embassy_rp::peripherals::PIN_22>,

    /// SDI pin.
    sdi: Output<'a, embassy_rp::peripherals::PIN_11>,

    /// CLK pin.
    clk: Output<'a, embassy_rp::peripherals::PIN_10>,

    /// LE pin.
    le: Output<'a, embassy_rp::peripherals::PIN_12>,
}

impl<'a> DisplayPins<'a> {
    /// Create a new display pins struct.
    pub fn new(
        a0: Output<'a, embassy_rp::peripherals::PIN_16>,
        a1: Output<'a, embassy_rp::peripherals::PIN_18>,
        a2: Output<'a, embassy_rp::peripherals::PIN_22>,
        sdi: Output<'a, embassy_rp::peripherals::PIN_11>,
        clk: Output<'a, embassy_rp::peripherals::PIN_10>,
        le: Output<'a, embassy_rp::peripherals::PIN_12>,
    ) -> Self {
        Self {
            a0,
            a1,
            a2,
            sdi,
            clk,
            le,
        }
    }
}

/// Update the display with accordance to the last known state of the matrix.
#[embassy_executor::task]
pub async fn update_matrix(mut pins: DisplayPins<'static>) {
    let mut row: usize = 0;

    loop {
        row = (row + 1) % 8;

        critical_section::with(|cs| {
            for col in display_matrix::DISPLAY_MATRIX.0.borrow_ref(cs)[row] {
                pins.clk.set_low();
                pins.sdi.set_low();

                if col == 1 {
                    pins.sdi.set_high();
                }

                pins.clk.set_high();
            }
        });

        pins.le.set_high();
        pins.le.set_low();

        if row & 0x01 != 0 {
            pins.a0.set_high();
        } else {
            pins.a0.set_low();
        }

        if row & 0x02 != 0 {
            pins.a1.set_high();
        } else {
            pins.a1.set_low();
        }

        if row & 0x04 != 0 {
            pins.a2.set_high();
        } else {
            pins.a2.set_low();
        }

        Timer::after(Duration::from_millis(1)).await;
    }
}

/// Backlight module. Will adjust backlight automatically.
pub mod backlight {
    use embassy_rp::{
        adc::{Adc, Async, Channel},
        gpio::Output,
    };
    use embassy_time::{Duration, Instant, Timer};

    use crate::config::{self, ReadAndSaveConfig};

    /// List of sleep durations, where higher numbers are brighter outputs.
    const LIGHT_LEVELS: [u64; 5] = [10, 100, 300, 700, 1000];

    /// All the pins required for backlight implementation.
    pub struct BacklightPins<'a> {
        /// OE pin.
        pub oe: Output<'static, embassy_rp::peripherals::PIN_13>,

        /// ADC controller.
        pub adc: Adc<'a, Async>,

        /// AIN pin.
        pub ain: Channel<'a>,
    }

    impl<'a> BacklightPins<'a> {
        /// Create a new backlight pins struct.
        pub fn new(
            oe: Output<'static, embassy_rp::peripherals::PIN_13>,
            adc: Adc<'a, Async>,
            ain: Channel<'a>,
        ) -> Self {
            Self { oe, adc, ain }
        }
    }

    /// Set brightness level every X seconds.
    #[embassy_executor::task]
    pub async fn update_backlight(mut pins: BacklightPins<'static>) {
        let mut last_backlight_read = Instant::now();
        let mut sleep_duration = LIGHT_LEVELS[3];

        loop {
            let now_time = Instant::now();
            if now_time.duration_since(last_backlight_read) >= Duration::from_secs(1)
                && config::CONFIG
                    .lock()
                    .await
                    .borrow_mut()
                    .as_mut()
                    .unwrap()
                    .get_autolight()
            {
                last_backlight_read = now_time;
                let level_read = pins.adc.read(&mut pins.ain).await.unwrap();
                sleep_duration = match level_read {
                    0..=3749 => LIGHT_LEVELS[4],
                    3750..=3799 => LIGHT_LEVELS[3],
                    3800..=3849 => LIGHT_LEVELS[2],
                    3850..=3899 => LIGHT_LEVELS[1],
                    _ => LIGHT_LEVELS[0],
                };
            }

            pins.oe.set_low();
            Timer::after(Duration::from_micros(sleep_duration)).await;
            pins.oe.set_high();
            Timer::after(Duration::from_micros(25)).await;
        }
    }
}

/// Display matrix module.
///
/// Contains all required data for updating state of waht to show on the display.
pub mod display_matrix {
    use chrono::Weekday;
    use embassy_futures::select::select;
    use embassy_sync::signal::Signal;
    use heapless::String;

    use crate::config::{TemperaturePreference, TimePreference};

    use super::*;

    /// Process the text buffer background task.
    ///
    /// Waits for text buffer to be updated and then will show the text. Each showing of the text can be cancelled by signalling the cancel signal.
    #[embassy_executor::task]
    pub async fn process_text_buffer() -> ! {
        loop {
            let item = TEXT_BUFFER.recv().await;

            CANCEL_SIGNAL.reset();

            select(DISPLAY_MATRIX.show_text(item), CANCEL_SIGNAL.wait()).await;
        }
    }

    /// The type of colon to use when showing the time.
    pub enum TimeColon {
        /// Display a full colon.
        Full,

        /// Display nothing.
        Empty,

        /// Display top half of a colon.
        Top,

        /// Display bottom half of a colon.
        Bottom,
    }

    /// Item to be added to the text buffer.
    struct TextBufferItem<'a> {
        /// A list of upto 32 [characters](Character).
        text: Vec<&'a Character<'a>, 32>,

        /// How long to hold on the dislay for in milliseconds once all text is shown.
        ///
        /// *This can be overridden by clearing the display queue so it is not a guarantee it will be on the display for this long*.
        hold_end_ms: u64,

        /// Where to start on the display.
        start_position: usize,

        /// Where to end on the display.
        end_position: usize,

        /// Scroll text off the display.
        scroll_off_display: bool,
    }

    /// Named struct for cancel signal.
    struct DisplayClearSignal;

    /// Text buffer channel. Can stored up to 16 elements in the queue.
    static TEXT_BUFFER: Channel<ThreadModeRawMutex, TextBufferItem<'_>, 16> = Channel::new();

    /// Cancel signal. Will cancel the current text being shown minimum wait.
    static CANCEL_SIGNAL: Signal<ThreadModeRawMutex, DisplayClearSignal> = Signal::new();

    /// Display matrix struct.
    pub struct DisplayMatrix(pub Mutex<RefCell<[[usize; 32]; 8]>>);

    /// Static access to display matrix. This should be used to modify the display.
    pub static DISPLAY_MATRIX: DisplayMatrix =
        DisplayMatrix(Mutex::new(RefCell::new([[0; 32]; 8])));

    impl DisplayMatrix {
        /// The first column after the icons.
        pub const DISPLAY_OFFSET: usize = 2;

        /// The last column that can be rendered.
        pub const LAST_INDEX: usize = 24;

        /// The delay between shifting the display items left.
        pub const SCROLL_DELAY: u64 = 150;

        /// Clear the entire display. Includes icons.
        ///
        /// # Arguments
        ///
        /// * `cs` - The critical section to access the display matrix.
        /// * `remove_queue` - Set true if you want to cancel the current display wait and remove all items in the text buffer queue.
        pub fn clear_all(&self, cs: CriticalSection, remove_queue: bool) {
            if remove_queue {
                Self::cancel_and_remove_queue();
            }

            self.0.replace(cs, [[0; 32]; 8]);
        }

        /// Clear the display. Does not include icons.
        ///
        /// # Arguments
        ///
        /// * `cs` - The critical section to access the display matrix.
        /// * `remove_queue` - Set true if you want to cancel the current display wait and remove all items in the text buffer queue.
        pub fn clear(&self, cs: CriticalSection, remove_queue: bool) {
            if remove_queue {
                Self::cancel_and_remove_queue();
            }

            let mut matrix = self.0.borrow_ref_mut(cs);

            for row in 1..8 {
                for col in 2..32 {
                    matrix[row][col] = 0;
                }
            }
        }

        /// Queue text into the text buffer. Will append to the queue.
        ///
        /// Will start at the display offset.
        /// Will end at the last index.
        ///
        /// Scrolling will be automatic if the text is too big to fit on the display.
        ///
        /// # Arguments
        ///
        /// * `text` - The text to show on the display.
        /// * `hold_end_ms` - Minimum period to show the text for.
        /// * `show_now` - Set true if you want to cancel the current display wait and remove all items in the text buffer queue.
        /// * `scroll_off_display` - Set true if you want the text to scroll off the display.
        pub async fn queue_text(
            &self,
            text: &str,
            hold_end_ms: u64,
            show_now: bool,
            scroll_off_display: bool,
        ) {
            if show_now {
                Self::cancel_and_remove_queue()
            }

            let mut final_text = text;
            if text.len() > 32 {
                final_text = &text[0..32];
            }

            let mut chars: Vec<&Character<'_>, 32> = Vec::new();

            for c in final_text.chars() {
                let character: Option<&Character> = get_character_struct(c);

                match character {
                    Some(ch) => {
                        chars.extend([ch]);
                    }
                    None => info!("Character {} not found", c),
                }
            }

            let buf = TextBufferItem {
                text: chars,
                hold_end_ms,
                start_position: Self::DISPLAY_OFFSET,
                end_position: Self::LAST_INDEX,
                scroll_off_display,
            };

            TEXT_BUFFER.send(buf).await;
        }

        /// Queue text into the text buffer. Will append to the queue.
        ///
        /// Will start at the `start_position`.
        /// Will end at the `LAST_INDEX`.
        ///
        /// Scrolling will be automatic if the text is too big to fit on the display.
        ///
        /// # Arguments
        ///
        /// * `start_position` - Where to start showing the text from.
        /// * `text` - The text to show on the display.
        /// * `hold_end_ms` - Minimum period to show the text for.
        /// * `show_now` - Set true if you want to cancel the current display wait and remove all items in the text buffer queue.
        pub async fn queue_text_from(
            &self,
            start_position: usize,
            text: &str,
            hold_end_ms: u64,
            show_now: bool,
        ) {
            if show_now {
                Self::cancel_and_remove_queue()
            }

            let mut final_text = text;
            if text.len() > 32 {
                final_text = &text[0..32];
            }

            let mut chars: Vec<&Character<'_>, 32> = Vec::new();

            for c in final_text.chars() {
                let character: Option<&Character> = get_character_struct(c);

                match character {
                    Some(ch) => {
                        chars.extend([ch]);
                    }
                    None => info!("Character {} not found", c),
                }
            }

            let buf = TextBufferItem {
                text: chars,
                hold_end_ms,
                start_position,
                end_position: Self::LAST_INDEX,
                scroll_off_display: false,
            };

            TEXT_BUFFER.send(buf).await;
        }

        /// Queue text into the text buffer. Will append to the queue.
        ///
        /// Will start at the `DISPLAY_OFFSET`.
        /// Will end at the `end_position`.
        ///
        /// Scrolling will be automatic if the text is too big to fit on the display.
        ///
        /// # Arguments
        ///
        /// * `end_position` - Where to end showing the text.
        /// * `text` - The text to show on the display.
        /// * `hold_end_ms` - Minimum period to show the text for.
        /// * `show_now` - Set true if you want to cancel the current display wait and remove all items in the text buffer queue.
        pub async fn queue_text_to(
            &self,
            end_position: usize,
            text: &str,
            hold_end_ms: u64,
            show_now: bool,
        ) {
            if show_now {
                Self::cancel_and_remove_queue()
            }

            let mut final_text = text;
            if text.len() > 32 {
                final_text = &text[0..32];
            }

            let mut chars: Vec<&Character<'_>, 32> = Vec::new();

            for c in final_text.chars() {
                let character: Option<&Character> = get_character_struct(c);

                match character {
                    Some(ch) => {
                        chars.extend([ch]);
                    }
                    None => info!("Character {} not found", c),
                }
            }

            let buf = TextBufferItem {
                text: chars,
                hold_end_ms,
                start_position: Self::DISPLAY_OFFSET,
                end_position,
                scroll_off_display: false,
            };

            TEXT_BUFFER.send(buf).await;
        }

        /// Queue the time into the text buffer. Will append to the queue.
        ///
        /// Will automatically prepend a 0 if any number is below 10.
        ///
        /// Can not scroll as the maximum buffer will not be exceeded.
        ///
        /// # Arguments
        ///
        /// * `left` - What to show on the left side of the `:`.
        /// * `right` - What to show on the right side of the `:`.
        /// * `colon` - What colon to show.
        /// * `hold_end_ms` - Minimum period to show the text for.
        /// * `show_now` - Set true if you want to cancel the current display wait and remove all items in the text buffer queue.
        /// * `scroll_off_display` - Set true if you want the text to scroll off the display.
        ///
        /// # Example
        ///
        /// ```rust
        /// DISPLAY_MATRIX.queue_time(10, 30, TimeColon::Full, 1000, false, false).await; // will render as 10:30 for at least 1 second.
        /// DISPLAY_MATRIX.queue_time(5, 5, TimeColon::Full, 1000, false, true).await; // will render as 05:05 for at least 1 second, then scroll all text off the display.
        /// ```
        pub async fn queue_time(
            &self,
            left: u32,
            right: u32,
            colon: TimeColon,
            hold_end_ms: u64,
            show_now: bool,
            scroll_off_display: bool,
        ) {
            let mut time = String::<8>::new();

            if left < 10 {
                _ = write!(time, "0{left}");
            } else {
                _ = write!(time, "{left}");
            }

            match colon {
                TimeColon::Full => _ = write!(time, ":"),
                TimeColon::Empty => _ = write!(time, " "),
                TimeColon::Top => _ = write!(time, "±"),
                TimeColon::Bottom => _ = write!(time, "§"),
            }

            if right < 10 {
                _ = write!(time, "0{right}");
            } else {
                _ = write!(time, "{right}");
            }

            self.queue_text(time.as_str(), hold_end_ms, show_now, scroll_off_display)
                .await;
        }

        /// Queue the time into the text buffer. Will append to the queue.
        ///
        /// Will automatically prepend a 0 if any number is below 10.
        ///
        /// Can not scroll as the maximum buffer will not be exceeded.
        ///
        /// # Arguments
        ///
        /// * `right` - What to show on the right side of the `:`.
        /// * `hold_end_ms` - Minimum period to show the text for.
        /// * `show_now` - Set true if you want to cancel the current display wait and remove all items in the text buffer queue.
        ///
        /// # Example
        ///
        /// ```rust
        /// DISPLAY_MATRIX.queue_time_left_side_blink(30, 1000, false).await; // will render as <>:30 for at least 1 second, where <> is empty space.
        /// DISPLAY_MATRIX.queue_time_left_side_blink(5, 1000, false).await; // will render as <>:05 for at least 1 second, where <> is empty space.
        /// ```
        pub async fn queue_time_left_side_blink(
            &self,
            right: u32,
            hold_end_ms: u64,
            show_now: bool,
        ) {
            let mut time = String::<8>::new();

            _ = write!(time, ":");

            if right < 10 {
                _ = write!(time, "0{right}");
            } else {
                _ = write!(time, "{right}");
            }

            self.queue_text_from(12, time.as_str(), hold_end_ms, show_now)
                .await;
        }

        /// Queue the time into the text buffer. Will append to the queue.
        ///
        /// Will automatically prepend a 0 if any number is below 10.
        ///
        /// Can not scroll as the maximum buffer will not be exceeded.
        ///
        /// # Arguments
        ///
        /// * `left` - What to show on the right side of the `:`.
        /// * `hold_end_ms` - Minimum period to show the text for.
        /// * `show_now` - Set true if you want to cancel the current display wait and remove all items in the text buffer queue.
        ///
        /// # Example
        ///
        /// ```rust
        /// DISPLAY_MATRIX.queue_time_right_side_blink(10, 1000, false).await; // will render as 10:<> for at least 1 second, where <> is empty space.
        /// DISPLAY_MATRIX.queue_time_right_side_blink(5, 1000, false).await; // will render as 05:<> for at least 1 second, where <> is empty space.
        /// ```
        pub async fn queue_time_right_side_blink(
            &self,
            left: u32,
            hold_end_ms: u64,
            show_now: bool,
        ) {
            let mut time = String::<8>::new();

            if left < 10 {
                _ = write!(time, "0{left}");
            } else {
                _ = write!(time, "{left}");
            }

            _ = write!(time, ":");

            self.queue_text_to(13, time.as_str(), hold_end_ms, show_now)
                .await;
        }

        /// Queue the year into the text buffer. Will append to the queue.
        ///
        /// Can not scroll as the maximum buffer will not be exceeded.
        ///
        /// # Arguments
        ///
        /// * `year` - What year to show.
        /// * `hold_end_ms` - Minimum period to show the text for.
        /// * `show_now` - Set true if you want to cancel the current display wait and remove all items in the text buffer queue.
        ///
        /// # Example
        ///
        /// ```rust
        /// DISPLAY_MATRIX.queue_year(2023, 1000, false).await; // will render as 2023 for at least 1 second.
        /// ```
        pub async fn queue_year(&self, year: i32, hold_end_ms: u64, show_now: bool) {
            let mut text: String<8> = String::<8>::new();

            _ = write!(text, "{year}");

            self.queue_text(text.as_str(), hold_end_ms, show_now, false)
                .await;
        }

        /// Queue the date into the text buffer. Will append to the queue.
        ///
        /// Will automatically prepend a 0 if any number is below 10.
        ///
        /// Can not scroll as the maximum buffer will not be exceeded.
        ///
        /// # Arguments
        ///
        /// * `left` - What to show on the left side of the `/`.
        /// * `right` - What to show on the right side of the `/`.
        /// * `hold_end_ms` - Minimum period to show the text for.
        /// * `show_now` - Set true if you want to cancel the current display wait and remove all items in the text buffer queue.
        ///
        /// # Example
        ///
        /// ```rust
        /// DISPLAY_MATRIX.queue_date(14, 12, 1000, false).await; // will render as 14:12 for at least 1 second.
        /// DISPLAY_MATRIX.queue_date(1, 12, 1000, false).await; // will render as 01:12 for at least 1 second.
        /// ```
        pub async fn queue_date(&self, left: u32, right: u32, hold_end_ms: u64, show_now: bool) {
            let mut date = String::<8>::new();

            if left < 10 {
                _ = write!(date, "0{left}");
            } else {
                _ = write!(date, "{left}");
            }

            _ = write!(date, "/");

            if right < 10 {
                _ = write!(date, "0{right}");
            } else {
                _ = write!(date, "{right}");
            }

            self.queue_text(date.as_str(), hold_end_ms, show_now, false)
                .await;
        }

        /// Queue the date into the text buffer. Will append to the queue.
        ///
        /// Will automatically prepend a 0 if any number is below 10.
        ///
        /// Can not scroll as the maximum buffer will not be exceeded.
        ///
        /// # Arguments
        ///
        /// * `right` - What to show on the right side of the `/`.
        /// * `hold_end_ms` - Minimum period to show the text for.
        /// * `show_now` - Set true if you want to cancel the current display wait and remove all items in the text buffer queue.
        ///
        /// # Example
        ///
        /// ```rust
        /// DISPLAY_MATRIX.queue_date_left_side_blink(14, 1000, false).await; // will render as <>/14 for at least 1 second, where <> is empty space.
        /// DISPLAY_MATRIX.queue_date_left_side_blink(1, 1000, false).await; // will render as <>/01 for at least 1 second, where <> is empty space.
        pub async fn queue_date_left_side_blink(
            &self,
            right: u32,
            hold_end_ms: u64,
            show_now: bool,
        ) {
            let mut time = String::<8>::new();

            _ = write!(time, "/");

            if right < 10 {
                _ = write!(time, "0{right}");
            } else {
                _ = write!(time, "{right}");
            }

            self.queue_text_from(12, time.as_str(), hold_end_ms, show_now)
                .await;
        }

        /// Queue the date into the text buffer. Will append to the queue.
        ///
        /// Will automatically prepend a 0 if any number is below 10.
        ///
        /// Can not scroll as the maximum buffer will not be exceeded.
        ///
        /// # Arguments
        ///
        /// * `left` - What to show on the left side of the `/`.
        /// * `hold_end_ms` - Minimum period to show the text for.
        /// * `show_now` - Set true if you want to cancel the current display wait and remove all items in the text buffer queue.
        ///
        /// # Example
        ///
        /// ```rust
        /// DISPLAY_MATRIX.queue_date_right_side_blink(12, 1000, false).await; // will render as 12/<> for at least 1 second, where <> is empty space.
        /// DISPLAY_MATRIX.queue_date_right_side_blink(1, 1000, false).await; // will render as 01/<> for at least 1 second, where <> is empty space.
        pub async fn queue_date_right_side_blink(
            &self,
            left: u32,
            hold_end_ms: u64,
            show_now: bool,
        ) {
            let mut time = String::<8>::new();

            if left < 10 {
                _ = write!(time, "0{left}");
            } else {
                _ = write!(time, "{left}");
            }

            _ = write!(time, "/");

            self.queue_text_to(13, time.as_str(), hold_end_ms, show_now)
                .await;
        }

        /// Queue the temperature into the text buffer. Will append to the queue.
        ///
        /// Will automatically add the appropriate temp symbol.
        ///
        /// # Arguments
        ///
        /// * `temp` - The temperature to show.
        /// * `pref` - What the temperature reporting preference is.
        /// * `show_now` - Set true if you want to cancel the current display wait and remove all items in the text buffer queue.
        /// * `scroll_off_display` - Set true if you want the text to scroll off the display.
        ///
        /// # Example
        ///
        /// ```rust
        /// DISPLAY_MATRIX.queue_temperature(25, TemperaturePreference::Celcius, false).await; // will render as 20°C.
        /// DISPLAY_MATRIX.queue_temperature(50, TemperaturePreference::Fahrenheit, true).await; // will render as 50°F and scroll off the display.
        pub async fn queue_temperature(
            &self,
            temp: f32,
            pref: TemperaturePreference,
            show_now: bool,
            scroll_off_display: bool,
        ) {
            let mut text = String::<8>::new();

            _ = write!(text, "{:.0}", temp);

            match pref {
                TemperaturePreference::Celcius => _ = write!(text, "°C"),
                TemperaturePreference::Fahrenheit => _ = write!(text, "°F"),
            }

            self.queue_text(text.as_str(), 2500, show_now, scroll_off_display)
                .await;
        }

        /// Queue the time and temperature into the text buffer. Will append to the queue.
        ///
        /// Will scroll the entire text base until it is empty.
        ///
        /// # Arguments
        ///
        /// * `hour` - The hour to show.
        /// * `min` - The minute to show.
        /// * `temp` - The temperature to show.
        /// * `pref` - What the temperature reporting preference is.
        /// * `show_now` - Set true if you want to cancel the current display wait and remove all items in the text buffer queue.
        ///
        /// # Example
        ///
        /// ```rust
        /// DISPLAY_MATRIX.queue_time_temperature(22, 10, 25, TemperaturePreference::Celcius, false).await; // will render as 22:10  20°C and scroll off the display.
        /// DISPLAY_MATRIX.queue_time_temperature(6, 30, 50, TemperaturePreference::Fahrenheit, true).await; // will render as 06:30  50°F and scroll off the display.
        pub async fn queue_time_temperature(
            &self,
            hour: u32,
            min: u32,
            temp: f32,
            pref: TemperaturePreference,
            show_now: bool,
        ) {
            let mut text = String::<16>::new();

            if hour < 10 {
                _ = write!(text, "0{hour}");
            } else {
                _ = write!(text, "{hour}");
            }

            _ = write!(text, ":");

            if min < 10 {
                _ = write!(text, "0{min}");
            } else {
                _ = write!(text, "{min}");
            }

            _ = write!(text, "  {:.0}", temp);

            match pref {
                TemperaturePreference::Celcius => _ = write!(text, "°C"),
                TemperaturePreference::Fahrenheit => _ = write!(text, "°F"),
            }

            self.queue_text(text.as_str(), 0, show_now, true).await;
        }

        /// Show text on the display. It will always clear what was shown previously.
        ///
        /// Responsible for moving items on the display left (animation) if the position of the last item is at the end of the display.
        async fn show_text(&self, item: TextBufferItem<'_>) {
            let mut total_width = 0;

            for c in &item.text {
                total_width += c.width;
                total_width += 1;
            }

            // if width is greater than matrix size with whitespace accounted for
            if total_width < Self::LAST_INDEX - 2 {
                critical_section::with(|cs| {
                    self.clear(cs, false);
                });
            }

            let mut pos = item.start_position;
            let space_char = get_character_struct('_').unwrap();
            for space in 2..pos {
                self.show_char(space_char, space).await;
            }

            for space in item.end_position..Self::LAST_INDEX {
                self.show_char(space_char, space).await;
            }

            for c in item.text {
                pos = self.show_char(c, pos).await;
                pos += 2;

                // if the position is greater than the last possible index and the total width is also greater (this won't be true for perfect fit items)
                if pos > Self::LAST_INDEX && total_width >= Self::LAST_INDEX {
                    self.shift_text_left(true);
                }
            }

            // set end hold to same time as scroll interval if scrolling off display
            // and hold is less than 0 (it looks jumpy otherwise)
            let hold_end_ms = if item.hold_end_ms < Self::SCROLL_DELAY && item.scroll_off_display {
                Self::SCROLL_DELAY
            } else {
                item.hold_end_ms
            };

            Timer::after(Duration::from_millis(hold_end_ms)).await;

            if item.scroll_off_display {
                while pos > Self::DISPLAY_OFFSET {
                    self.shift_text_left(false);
                    Timer::after(Duration::from_millis(Self::SCROLL_DELAY)).await;
                    pos -= 1;
                }
            }
        }

        /// Show an individual [character](Character) at the given position.
        ///
        /// Will move the display left (animation) if the column exceeds the `LAST_INDEX`.
        ///
        /// Returns the last column populated by the character.
        async fn show_char(&self, character: &Character<'_>, mut pos: usize) -> usize {
            let mut matrix = critical_section::with(|cs| *self.0.borrow_ref(cs));

            let first_pos = pos;
            let mut hit_end_of_display = false;

            for col in 0..*character.width {
                pos = first_pos + col;

                if pos > Self::LAST_INDEX {
                    // if first time hitting end of display, pause for better readability
                    if !hit_end_of_display {
                        Timer::after(Duration::from_millis(Self::SCROLL_DELAY)).await;
                        hit_end_of_display = true;
                    }

                    pos = Self::LAST_INDEX;

                    self.shift_text_left(false);

                    Timer::after(Duration::from_millis(Self::SCROLL_DELAY)).await;

                    // grab matrix again after update
                    matrix = critical_section::with(|cs| *self.0.borrow_ref(cs));
                }

                for (row, item) in matrix.iter_mut().enumerate().skip(1) {
                    let byte = character.values[row - 1];
                    item[pos] = (byte >> col) % 2;
                }

                critical_section::with(|cs| self.0.replace(cs, matrix));
            }

            pos
        }

        /// Show an icon on the display.
        ///
        /// `icon_text` should be a string that can be returned from the [lookup table fn](get_character_struct).
        ///
        /// Will do nothing if the icon is already displayed or the icon can not be found.
        pub fn show_icon(&self, icon_text: &str) {
            critical_section::with(|cs| {
                let mut matrix = self.0.borrow_ref_mut(cs);

                let icon: Option<&Icon> = get_icon_struct(icon_text);
                match icon {
                    Some(i) => {
                        for w in 0..i.width {
                            matrix[i.col][i.row + w] = 1;
                        }
                    }
                    None => info!("Icon {} not found", icon_text),
                }
            })
        }

        /// Hide an icon on the display.
        ///
        /// `icon_text` should be a string that can be returned from the [lookup table fn](get_character_struct).
        ///
        /// Will do nothing if the icon is already displayed or the icon can not be found.
        pub fn hide_icon(&self, icon_text: &str) {
            critical_section::with(|cs| {
                let mut matrix = self.0.borrow_ref_mut(cs);

                let icon: Option<&Icon> = get_icon_struct(icon_text);
                match icon {
                    Some(i) => {
                        for w in 0..i.width {
                            matrix[i.col][i.row + w] = 0;
                        }
                    }
                    None => info!("Icon {} not found", icon_text),
                }
            })
        }

        /// Show a day icon, determined from `day`.
        ///
        /// **This is intended for use during normal function where days are incremented at 12am. It will only hide the previous day icon, not all other days.**
        pub fn show_day_icon(&self, day: Weekday) {
            match day {
                Weekday::Mon => {
                    self.hide_icon("Sun");
                    self.show_icon("Mon");
                }
                Weekday::Tue => {
                    self.hide_icon("Mon");
                    self.show_icon("Tue");
                }
                Weekday::Wed => {
                    self.hide_icon("Tue");
                    self.show_icon("Wed");
                }
                Weekday::Thu => {
                    self.hide_icon("Wed");
                    self.show_icon("Thur");
                }
                Weekday::Fri => {
                    self.hide_icon("Thur");
                    self.show_icon("Fri");
                }
                Weekday::Sat => {
                    self.hide_icon("Fri");
                    self.show_icon("Sat");
                }
                Weekday::Sun => {
                    self.hide_icon("Sat");
                    self.show_icon("Sun");
                }
            }
        }

        /// Show the correct temperature preference icon.
        pub fn show_temperature_icon(&self, pref: TemperaturePreference) {
            match pref {
                TemperaturePreference::Celcius => {
                    self.hide_icon("°F");
                    self.show_icon("°C");
                }
                TemperaturePreference::Fahrenheit => {
                    self.hide_icon("°C");
                    self.show_icon("°F");
                }
            }
        }

        /// Show the correct temperature preference icon.
        pub fn show_time_icon(&self, pref: TimePreference, hour: u32) {
            match pref {
                TimePreference::Twelve => {
                    if hour >= 12 {
                        self.hide_icon("AM");
                        self.show_icon("PM");
                    } else {
                        self.hide_icon("PM");
                        self.show_icon("AM");
                    }
                }
                TimePreference::TwentyFour => {
                    self.hide_icon("AM");
                    self.hide_icon("PM");
                }
            }
        }

        /// Show or hide the autolight icon.
        pub fn show_autolight_icon(&self, state: bool) {
            if state {
                self.show_icon("AutoLight");
            } else {
                self.hide_icon("AutoLight");
            }
        }

        /// Move items in the column left by one space. Will add a blank space at the end of the display if `add_space` is true.
        fn shift_text_left(&self, add_space: bool) {
            let mut matrix = critical_section::with(|cs| *self.0.borrow_ref(cs));

            // skip day of week icons
            for item in matrix.iter_mut().skip(1) {
                // start from here to account for icon width buffer
                for col in 4..32 {
                    item[col - 2] = item[col - 1];
                    if add_space {
                        item[col - 1] = 0;
                    }
                }
            }

            critical_section::with(|cs| self.0.replace(cs, matrix));
        }

        /// Cancel the current minimum display task and clear the text buffer.
        fn cancel_and_remove_queue() {
            CANCEL_SIGNAL.signal(DisplayClearSignal);

            // text buffer does not have clear, so create loop that runs until try_recv fails, then break
            loop {
                let res = TEXT_BUFFER.try_recv();
                match res {
                    Ok(_) => {}
                    Err(_) => break,
                }
            }

            critical_section::with(|cs| {
                DISPLAY_MATRIX.clear(cs, false);
            });
        }
    }
}

/// Module for handling text on the display.
mod text {
    /// Represent text display on the display.
    #[derive(Clone)]
    pub struct Character<'a> {
        /// The width of the character.
        pub width: &'a usize,

        /// The hex representation for each row and column.
        pub values: &'a [usize],
    }

    impl<'a> Character<'a> {
        /// Create a new character.
        const fn new(width: &'a usize, values: &'a [usize]) -> Self {
            Self { width, values }
        }
    }

    /// All supported characters lookup table.
    const CHARACTER_TABLE: [(char, Character); 46] = [
        (
            '0',
            Character::new(&4, &[0x06, 0x09, 0x09, 0x09, 0x09, 0x09, 0x06]),
        ),
        (
            '1',
            Character::new(&4, &[0x04, 0x06, 0x04, 0x04, 0x04, 0x04, 0x0E]),
        ),
        (
            '2',
            Character::new(&4, &[0x06, 0x09, 0x08, 0x04, 0x02, 0x01, 0x0F]),
        ),
        (
            '3',
            Character::new(&4, &[0x06, 0x09, 0x08, 0x06, 0x08, 0x09, 0x06]),
        ),
        (
            '4',
            Character::new(&4, &[0x08, 0x0C, 0x0A, 0x09, 0x0F, 0x08, 0x08]),
        ),
        (
            '5',
            Character::new(&4, &[0x0F, 0x01, 0x07, 0x08, 0x08, 0x09, 0x06]),
        ),
        (
            '6',
            Character::new(&4, &[0x04, 0x02, 0x01, 0x07, 0x09, 0x09, 0x06]),
        ),
        (
            '7',
            Character::new(&4, &[0x0F, 0x09, 0x04, 0x04, 0x04, 0x04, 0x04]),
        ),
        (
            '8',
            Character::new(&4, &[0x06, 0x09, 0x09, 0x06, 0x09, 0x09, 0x06]),
        ),
        (
            '9',
            Character::new(&4, &[0x06, 0x09, 0x09, 0x0E, 0x08, 0x04, 0x02]),
        ),
        (
            'A',
            Character::new(&4, &[0x06, 0x09, 0x09, 0x0F, 0x09, 0x09, 0x09]),
        ),
        (
            'B',
            Character::new(&4, &[0x07, 0x09, 0x09, 0x07, 0x09, 0x09, 0x07]),
        ),
        (
            'C',
            Character::new(&4, &[0x06, 0x09, 0x01, 0x01, 0x01, 0x09, 0x06]),
        ),
        (
            'D',
            Character::new(&4, &[0x07, 0x09, 0x09, 0x09, 0x09, 0x09, 0x07]),
        ),
        (
            'E',
            Character::new(&4, &[0x0F, 0x01, 0x01, 0x0F, 0x01, 0x01, 0x0F]),
        ),
        (
            'F',
            Character::new(&4, &[0x0F, 0x01, 0x01, 0x0F, 0x01, 0x01, 0x01]),
        ),
        (
            'G',
            Character::new(&4, &[0x06, 0x09, 0x01, 0x0D, 0x09, 0x09, 0x06]),
        ),
        (
            'H',
            Character::new(&4, &[0x09, 0x09, 0x09, 0x0F, 0x09, 0x09, 0x09]),
        ),
        (
            'I',
            Character::new(&3, &[0x07, 0x02, 0x02, 0x02, 0x02, 0x02, 0x07]),
        ),
        (
            'J',
            Character::new(&4, &[0x0F, 0x08, 0x08, 0x08, 0x09, 0x09, 0x06]),
        ),
        (
            'K',
            Character::new(&4, &[0x09, 0x05, 0x03, 0x01, 0x03, 0x05, 0x09]),
        ),
        (
            'L',
            Character::new(&4, &[0x01, 0x01, 0x01, 0x01, 0x01, 0x01, 0x0F]),
        ),
        (
            'M',
            Character::new(&5, &[0x11, 0x1B, 0x15, 0x11, 0x11, 0x11, 0x11]),
        ),
        (
            'N',
            Character::new(&4, &[0x09, 0x09, 0x0B, 0x0D, 0x09, 0x09, 0x09]),
        ),
        (
            'O',
            Character::new(&4, &[0x06, 0x09, 0x09, 0x09, 0x09, 0x09, 0x06]),
        ),
        (
            'P',
            Character::new(&4, &[0x07, 0x09, 0x09, 0x07, 0x01, 0x01, 0x01]),
        ),
        (
            'Q',
            Character::new(&5, &[0x0E, 0x11, 0x11, 0x11, 0x15, 0x19, 0x0E]),
        ),
        (
            'R',
            Character::new(&4, &[0x07, 0x09, 0x09, 0x07, 0x03, 0x05, 0x09]),
        ),
        (
            'S',
            Character::new(&4, &[0x06, 0x09, 0x02, 0x04, 0x08, 0x09, 0x06]),
        ),
        (
            'T',
            Character::new(&5, &[0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04]),
        ),
        (
            'U',
            Character::new(&4, &[0x09, 0x09, 0x09, 0x09, 0x09, 0x09, 0x06]),
        ),
        (
            'V',
            Character::new(&5, &[0x11, 0x11, 0x11, 0x11, 0x11, 0x0A, 0x04]),
        ),
        (
            'W',
            Character::new(&5, &[0x11, 0x11, 0x11, 0x15, 0x15, 0x1B, 0x11]),
        ),
        (
            'X',
            Character::new(&5, &[0x11, 0x0A, 0x04, 0x04, 0x04, 0x0A, 0x11]),
        ),
        (
            'Y',
            Character::new(&5, &[0x11, 0x11, 0x0A, 0x04, 0x04, 0x04, 0x04]),
        ),
        (
            'Z',
            Character::new(&4, &[0x0F, 0x08, 0x04, 0x02, 0x01, 0x0F, 0x00]),
        ),
        (
            ':',
            Character::new(&2, &[0x00, 0x03, 0x03, 0x00, 0x03, 0x03, 0x00]),
        ),
        // top half of a : only
        (
            '±',
            Character::new(&2, &[0x00, 0x03, 0x03, 0x00, 0x00, 0x00, 0x00]),
        ),
        // bottom half of a : only
        (
            '§',
            Character::new(&2, &[0x00, 0x00, 0x00, 0x00, 0x03, 0x03, 0x00]),
        ),
        (
            ' ',
            Character::new(&2, &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        ),
        (
            '°',
            Character::new(&2, &[0x03, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00]),
        ),
        (
            '.',
            Character::new(&1, &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01]),
        ),
        (
            '-',
            Character::new(&2, &[0x00, 0x00, 0x00, 0x03, 0x00, 0x00, 0x00]),
        ),
        (
            '/',
            Character::new(&2, &[0x02, 0x02, 0x02, 0x01, 0x01, 0x01, 0x01, 0x01]),
        ),
        (
            '+',
            Character::new(&5, &[0x00, 0x04, 0x04, 0x1F, 0x04, 0x04, 0x00]),
        ),
        // empty space
        (
            '_',
            Character::new(&1, &[0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]),
        ),
    ];

    /// Find the [character](Character) for the `character` param.
    ///
    /// Will return [None](Option::None) if the icon is not found in the [lookup table](CHARACTER_TABLE).
    ///
    /// # Example
    /// ```rust
    /// let char_text = 'A';
    /// let ch: Option<&Character> = get_character_struct(char_text);
    /// match ch {
    ///     Some(c) => info!("Character {} found!", char_text),
    ///     None => info!("Character {} not found", char_text),
    /// }
    /// // prints: Character A found!
    /// ```
    pub fn get_character_struct(character: char) -> Option<&'static Character<'static>> {
        for &(c, ref info) in &CHARACTER_TABLE {
            if c == character.to_ascii_uppercase() {
                return Some(info);
            }
        }
        None
    }
}

/// Module for handling icons on the display.
mod icons {
    /// Represent an icon on the display.
    pub struct Icon {
        /// The row the icon is on.
        pub row: usize,

        /// The column the icon starts on. `width` is used to determine where it ends.
        pub col: usize,

        /// The width of the icon. This will be either 1 or 2.
        pub width: usize,
    }

    impl Icon {
        /// Create a new icon representation.
        const fn new(x: usize, y: usize, width: usize) -> Icon {
            Self {
                row: x,
                col: y,
                width,
            }
        }
    }

    /// All icons lookup table.
    pub const ICON_TABLE: [(&str, Icon); 17] = [
        ("MoveOn", Icon::new(0, 0, 2)),
        ("AlarmOn", Icon::new(0, 1, 2)),
        ("CountDown", Icon::new(0, 2, 2)),
        ("°F", Icon::new(0, 3, 1)),
        ("°C", Icon::new(1, 3, 1)),
        ("AM", Icon::new(0, 4, 1)),
        ("PM", Icon::new(1, 4, 1)),
        ("CountUp", Icon::new(0, 5, 2)),
        ("Hourly", Icon::new(0, 6, 2)),
        ("AutoLight", Icon::new(0, 7, 2)),
        ("Mon", Icon::new(3, 0, 2)),
        ("Tue", Icon::new(6, 0, 2)),
        ("Wed", Icon::new(9, 0, 2)),
        ("Thur", Icon::new(12, 0, 2)),
        ("Fri", Icon::new(15, 0, 2)),
        ("Sat", Icon::new(18, 0, 2)),
        ("Sun", Icon::new(21, 0, 2)),
    ];

    /// Find the [icon](Icon) for the `icon` param.
    ///
    /// Will return [None](Option::None) if the icon is not found in the [lookup table](ICON_TABLE).
    ///
    /// # Example
    /// ```rust
    /// let icon_text = "MoveOn";
    /// let icon: Option<&Icon> = get_icon_struct(icon_text);
    /// match icon {
    ///     Some(i) => info!("Icon {} found!", icon_text),
    ///     None => info!("Icon {} not found", icon_text),
    /// }
    ///
    /// // prints: Icon MoveOn found!
    /// ```
    pub fn get_icon_struct(icon: &str) -> Option<&Icon> {
        for &(c, ref info) in &ICON_TABLE {
            if c == icon {
                return Some(info);
            }
        }
        None
    }
}
