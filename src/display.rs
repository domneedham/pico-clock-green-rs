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

pub struct DisplayPins<'a> {
    a0: Output<'a, embassy_rp::peripherals::PIN_16>,
    a1: Output<'a, embassy_rp::peripherals::PIN_18>,
    a2: Output<'a, embassy_rp::peripherals::PIN_22>,
    oe: Output<'a, embassy_rp::peripherals::PIN_13>,
    sdi: Output<'a, embassy_rp::peripherals::PIN_11>,
    clk: Output<'a, embassy_rp::peripherals::PIN_10>,
    le: Output<'a, embassy_rp::peripherals::PIN_12>,
}

impl<'a> DisplayPins<'a> {
    pub fn new(
        a0: Output<'a, embassy_rp::peripherals::PIN_16>,
        a1: Output<'a, embassy_rp::peripherals::PIN_18>,
        a2: Output<'a, embassy_rp::peripherals::PIN_22>,
        oe: Output<'a, embassy_rp::peripherals::PIN_13>,
        sdi: Output<'a, embassy_rp::peripherals::PIN_11>,
        clk: Output<'a, embassy_rp::peripherals::PIN_10>,
        le: Output<'a, embassy_rp::peripherals::PIN_12>,
    ) -> Self {
        Self {
            a0,
            a1,
            a2,
            oe,
            sdi,
            clk,
            le,
        }
    }
}

pub struct Display<'a> {
    pins: DisplayPins<'a>,
    row: usize,
}

impl<'a> Display<'a> {
    pub fn new(pins: DisplayPins<'a>) -> Display {
        Self { pins, row: 0 }
    }

    pub async fn run_forever(&mut self) -> ! {
        loop {
            self.row = (self.row + 1) % 8;

            critical_section::with(|cs| {
                for col in display_matrix::DISPLAY_MATRIX.0.borrow_ref(cs)[self.row] {
                    self.pins.clk.set_low();
                    if col == 1 {
                        self.pins.sdi.set_high();
                    } else {
                        self.pins.sdi.set_low();
                    }
                    self.pins.clk.set_high();
                }
            });

            self.pins.le.set_high();
            self.pins.le.set_low();

            if self.row & 0x01 != 0 {
                self.pins.a0.set_high();
            } else {
                self.pins.a0.set_low();
            }

            if self.row & 0x02 != 0 {
                self.pins.a1.set_high();
            } else {
                self.pins.a1.set_low();
            }

            if self.row & 0x04 != 0 {
                self.pins.a2.set_high();
            } else {
                self.pins.a2.set_low();
            }

            self.pins.oe.set_low();
            Timer::after(Duration::from_micros(100)).await;
            self.pins.oe.set_high();
        }
    }
}

pub mod display_matrix {
    use chrono::Weekday;
    use embassy_futures::select::select;
    use embassy_sync::signal::Signal;
    use heapless::String;

    use super::*;

    #[embassy_executor::task]
    pub async fn process_text_buffer() -> ! {
        loop {
            let item = TEXT_BUFFER.recv().await;

            CANCEL_SIGNAL.reset();

            select(DISPLAY_MATRIX.show_text(item), CANCEL_SIGNAL.wait()).await;
        }
    }

    struct TextBufferItem<'a> {
        text: Vec<&'a Character<'a>, 32>,
        hold_s: u64,
    }

    struct DisplayClearSignal();

    static TEXT_BUFFER: Channel<ThreadModeRawMutex, TextBufferItem<'_>, 16> = Channel::new();
    static CANCEL_SIGNAL: Signal<ThreadModeRawMutex, DisplayClearSignal> = Signal::new();

    pub struct DisplayMatrix(pub Mutex<RefCell<[[usize; 32]; 8]>>);

    pub static DISPLAY_MATRIX: DisplayMatrix =
        DisplayMatrix(Mutex::new(RefCell::new([[0; 32]; 8])));

    impl DisplayMatrix {
        const DISPLAY_OFFSET: usize = 2;
        const LAST_INDEX: usize = 24;

        pub fn clear_all(&self, cs: CriticalSection, remove_queue: bool) {
            if remove_queue {
                Self::cancel_and_remove_queue();
            }

            self.0.replace(cs, [[0; 32]; 8]);
        }

        pub fn fill_all(&self, cs: CriticalSection, remove_queue: bool) {
            if remove_queue {
                Self::cancel_and_remove_queue();
            }

            self.0.replace(cs, [[1; 32]; 8]);
        }

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

        pub async fn test_text(&self) {
            self.queue_text("HELLO WORLD", true).await;
        }

        pub async fn queue_text(&self, text: &str, show_now: bool) {
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
                hold_s: 1,
            };

            TEXT_BUFFER.send(buf).await;
        }

        pub async fn queue_time(&self, left: u32, right: u32, show_now: bool) {
            let mut time = String::<8>::new();

            if left < 10 {
                _ = write!(time, "0{left}");
            } else {
                _ = write!(time, "{left}");
            }

            _ = write!(time, ":");

            if right < 10 {
                _ = write!(time, "0{right}");
            } else {
                _ = write!(time, "{right}");
            }

            self.queue_text(time.as_str(), show_now).await;
        }

        async fn show_text(&self, item: TextBufferItem<'_>) {
            critical_section::with(|cs| {
                self.clear(cs, false);
            });

            let mut total_width = 0;

            for c in &item.text {
                total_width += c.width;
            }

            let mut pos = Self::DISPLAY_OFFSET;
            for c in item.text {
                pos = self.show_char(c, pos).await;
                pos += 2;

                // if the position is greater than the last possible index and the total width is also greater (this won't be true for perfect fit items)
                if pos >= Self::LAST_INDEX && total_width > Self::LAST_INDEX {
                    self.shift_text_left(true);
                }
            }

            Timer::after(Duration::from_secs(item.hold_s)).await;
        }

        async fn show_char(&self, character: &Character<'_>, mut pos: usize) -> usize {
            let mut matrix = critical_section::with(|cs| *self.0.borrow_ref(cs));

            let first_pos = pos;
            let mut hit_end_of_display = false;

            for col in 0..*character.width {
                pos = first_pos + col;

                if pos > Self::LAST_INDEX {
                    // if first time hitting end of display, pause for better readability
                    if !hit_end_of_display {
                        Timer::after(Duration::from_millis(300)).await;
                        hit_end_of_display = true;
                    }

                    pos = Self::LAST_INDEX;

                    self.shift_text_left(false);

                    Timer::after(Duration::from_millis(300)).await;

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

        pub fn test_icons(&self, cs: CriticalSection) {
            self.show_icon(cs, "AutoLight");
            self.show_icon(cs, "Tue");
            self.hide_icon(cs, "Tue");
            self.show_icon(cs, "Mon");
        }

        pub fn show_icon(&self, cs: CriticalSection, icon_text: &str) {
            let mut matrix = self.0.borrow_ref_mut(cs);

            let icon: Option<&Icon> = get_icon_struct(icon_text);
            match icon {
                Some(i) => {
                    for w in 0..i.width {
                        matrix[i.y][i.x + w] = 1;
                    }
                }
                None => info!("Icon {} not found", icon_text),
            }
        }

        pub fn hide_icon(&self, cs: CriticalSection, icon_text: &str) {
            let mut matrix = self.0.borrow_ref_mut(cs);

            let icon: Option<&Icon> = get_icon_struct(icon_text);
            match icon {
                Some(i) => {
                    for w in 0..i.width {
                        matrix[i.y][i.x + w] = 0;
                    }
                }
                None => info!("Icon {} not found", icon_text),
            }
        }

        pub fn show_day_icon(&self, day: Weekday) {
            critical_section::with(|cs| match day {
                Weekday::Mon => {
                    self.hide_icon(cs, "Sun");
                    self.show_icon(cs, "Mon");
                }
                Weekday::Tue => {
                    self.hide_icon(cs, "Mon");
                    self.show_icon(cs, "Tue");
                }
                Weekday::Wed => {
                    self.hide_icon(cs, "Tue");
                    self.show_icon(cs, "Wed");
                }
                Weekday::Thu => {
                    self.hide_icon(cs, "Wed");
                    self.show_icon(cs, "Thur");
                }
                Weekday::Fri => {
                    self.hide_icon(cs, "Thur");
                    self.show_icon(cs, "Fri");
                }
                Weekday::Sat => {
                    self.hide_icon(cs, "Fri");
                    self.show_icon(cs, "Sat");
                }
                Weekday::Sun => {
                    self.hide_icon(cs, "Sat");
                    self.show_icon(cs, "Sun");
                }
            })
        }

        fn shift_text_left(&self, add_space: bool) {
            let mut matrix = critical_section::with(|cs| *self.0.borrow_ref(cs));

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

        fn cancel_and_remove_queue() {
            CANCEL_SIGNAL.signal(DisplayClearSignal());

            loop {
                let res = TEXT_BUFFER.try_recv();
                match res {
                    Ok(_) => {}
                    Err(_) => break,
                }
            }
        }
    }
}

mod text {
    #[derive(Clone)]
    pub struct Character<'a> {
        pub width: &'a usize,
        pub values: &'a [usize],
    }

    impl<'a> Character<'a> {
        const fn new(width: &'a usize, values: &'a [usize]) -> Self {
            Self { width, values }
        }
    }

    const CHARACTER_TABLE: [(char, Character); 42] = [
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
            Character::new(&4, &[0x1F, 0x04, 0x04, 0x04, 0x04, 0x04, 0x04]),
        ),
        (
            'Z',
            Character::new(&4, &[0x0F, 0x08, 0x04, 0x02, 0x01, 0x0F, 0x00]),
        ),
        (
            ':',
            Character::new(&2, &[0x00, 0x03, 0x03, 0x00, 0x03, 0x03, 0x00]),
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
    ];

    pub fn get_character_struct(character: char) -> Option<&'static Character<'static>> {
        for &(c, ref info) in &CHARACTER_TABLE {
            if c == character.to_ascii_uppercase() {
                return Some(info);
            }
        }
        None
    }
}

mod icons {
    pub struct Icon {
        pub x: usize,
        pub y: usize,
        pub width: usize,
    }

    impl Icon {
        const fn new(x: usize, y: usize, width: usize) -> Icon {
            Self { x, y, width }
        }
    }

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

    pub fn get_icon_struct(icon: &str) -> Option<&Icon> {
        for &(c, ref info) in &ICON_TABLE {
            if c == icon {
                return Some(info);
            }
        }
        None
    }
}
