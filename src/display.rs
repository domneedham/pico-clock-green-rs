use core::cell::RefCell;
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

    pub async fn update_display(&mut self) {
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

pub mod display_matrix {
    use embassy_futures::select::select;
    use embassy_futures::select::Either::{First, Second};
    use embassy_sync::signal::Signal;

    use super::*;

    #[embassy_executor::task]
    pub async fn process_text_buffer() -> ! {
        loop {
            let item = TEXT_BUFFER.recv().await;

            CANCEL_SIGNAL.reset();

            let completed_future =
                select(DISPLAY_MATRIX.show_text(item), CANCEL_SIGNAL.wait()).await;

            match completed_future {
                First(_) => info!("Completed"),
                Second(_) => info!("Task cancelled"),
            };
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
        DisplayMatrix(Mutex::new(RefCell::new([[1; 32]; 8])));

    impl DisplayMatrix {
        const DISPLAY_OFFSET: usize = 2;

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
            self.queue_text("21:11", false).await;
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

        async fn show_text(&self, item: TextBufferItem<'_>) {
            critical_section::with(|cs| {
                self.clear(cs, false);
            });

            let mut total_width = 0;

            for c in &item.text {
                total_width += c.width;
            }

            let mut pos = 0;
            for c in &item.text {
                self.show_char(c, pos).await;
                pos += c.width + 1; // add column space between characters
            }

            Timer::after(Duration::from_secs(item.hold_s)).await;
        }

        async fn show_char(&self, character: &Character<'_>, mut pos: usize) {
            let mut matrix = critical_section::with(|cs| *self.0.borrow_ref(cs));

            pos += Self::DISPLAY_OFFSET; // Plus the offset of the status indicator

            let mut animated = false;

            for row in 1..8 {
                let byte = character.values[row - 1];
                for col in 0..*character.width {
                    let mut c = pos + col;

                    if c >= 28 - *character.width {
                        if !animated {
                            Timer::after(Duration::from_millis(250)).await;
                            animated = true;
                        }

                        for column in 3..32 {
                            matrix[row][column - 1] = matrix[row][column];
                        }

                        critical_section::with(|cs| self.0.replace(cs, matrix));

                        c = 27 - *character.width;
                    }

                    matrix[row][c] = (byte >> col) % 2;
                }

                // if hit end of the display, add a space at the end of the character
                if animated {
                    for column in 3..32 {
                        matrix[row][column - 1] = matrix[row][column];
                        matrix[row][column] = 0;
                    }
                }

                critical_section::with(|cs| self.0.replace(cs, matrix));
            }

            if animated {
                Timer::after(Duration::from_millis(250)).await;
            }
        }

        pub fn test_icons(&self, cs: CriticalSection) {
            self.show_icon(cs, "AutoLight");
            self.show_icon(cs, "Tue")
        }

        pub fn show_icon(&self, cs: CriticalSection, icon_text: &'static str) {
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

    pub fn get_icon_struct(icon: &'static str) -> Option<&Icon> {
        for &(c, ref info) in &ICON_TABLE {
            if c == icon {
                return Some(info);
            }
        }
        None
    }
}
