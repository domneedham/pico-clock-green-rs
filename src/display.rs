use core::cell::RefCell;

use cortex_m::delay::Delay;
use critical_section::{CriticalSection, Mutex};
use defmt::info;
use embedded_hal::digital::v2::OutputPin;
use rp_pico::hal::gpio::{bank0::*, Output, Pin, PushPull};

use self::{
    icons::{get_icon_struct, Icon},
    text::{get_character_struct, Character},
};

pub struct DisplayMatrix(Mutex<RefCell<[[usize; 32]; 8]>>);

impl DisplayMatrix {
    const DISPLAY_OFFSET: usize = 2;

    pub fn clear(&self, cs: CriticalSection) {
        self.0.replace(cs, [[0; 32]; 8]);
    }

    pub fn fill(&self, cs: CriticalSection) {
        self.0.replace(cs, [[1; 32]; 8]);
    }

    pub fn test_text(&self, cs: CriticalSection) {
        self.show_text(cs, "hi", false);
    }

    pub fn show_text(&self, cs: CriticalSection, text: &str, clear: bool) {
        if clear {
            self.clear(cs);
        }

        let mut pos = 0;
        for c in text.chars() {
            let character: Option<&Character> = get_character_struct(c);
            match character {
                Some(ch) => {
                    self.show_char(cs, ch, pos);
                    pos += ch.width + 1; // add column space between characters
                }
                None => info!("Letter {} not found", c),
            }
        }
    }

    fn show_char(&self, cs: CriticalSection, character: &Character, mut pos: usize) {
        let mut matrix = self.0.borrow_ref_mut(cs);

        pos += Self::DISPLAY_OFFSET; // Plus the offset of the status indicator

        for row in 1..8 {
            let byte = character.values[row - 1];
            for col in 0..*character.width {
                let c = pos + col;
                matrix[row][c] = (byte >> col) % 2;
            }
        }
    }

    pub fn test_icons(&self, cs: CriticalSection) {
        self.show_icon(cs, "AutoLight");
        self.show_icon(cs, "Sat")
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
}

const DISPLAY_MATRIX_INIT: DisplayMatrix = DisplayMatrix(Mutex::new(RefCell::new([[1; 32]; 8])));

pub static DISPLAY_MATRIX: DisplayMatrix = DISPLAY_MATRIX_INIT;

pub struct DisplayPins {
    a0: Pin<Gpio16, Output<PushPull>>,
    a1: Pin<Gpio18, Output<PushPull>>,
    a2: Pin<Gpio22, Output<PushPull>>,
    oe: Pin<Gpio13, Output<PushPull>>,
    sdi: Pin<Gpio11, Output<PushPull>>,
    clk: Pin<Gpio10, Output<PushPull>>,
    le: Pin<Gpio12, Output<PushPull>>,
}

impl DisplayPins {
    pub fn new(
        a0: Pin<Gpio16, Output<PushPull>>,
        a1: Pin<Gpio18, Output<PushPull>>,
        a2: Pin<Gpio22, Output<PushPull>>,
        oe: Pin<Gpio13, Output<PushPull>>,
        sdi: Pin<Gpio11, Output<PushPull>>,
        clk: Pin<Gpio10, Output<PushPull>>,
        le: Pin<Gpio12, Output<PushPull>>,
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

pub struct Display {
    pins: DisplayPins,
    row: usize,
}

impl<'a> Display {
    pub fn new(pins: DisplayPins) -> Display {
        Self { pins, row: 0 }
    }

    pub fn update_display(&mut self, mut delay: Delay) -> ! {
        loop {
            self.row = (self.row + 1) % 8;

            critical_section::with(|cs| {
                for col in DISPLAY_MATRIX.0.borrow_ref(cs)[self.row] {
                    self.pins.clk.set_low().unwrap();
                    if col == 1 {
                        self.pins.sdi.set_high().unwrap();
                    } else {
                        self.pins.sdi.set_low().unwrap();
                    }
                    self.pins.clk.set_high().unwrap();
                }
            });

            self.pins.le.set_high().unwrap();
            self.pins.le.set_low().unwrap();

            if self.row & 0x01 != 0 {
                self.pins.a0.set_high().unwrap();
            } else {
                self.pins.a0.set_low().unwrap();
            }

            if self.row & 0x02 != 0 {
                self.pins.a1.set_high().unwrap();
            } else {
                self.pins.a1.set_low().unwrap();
            }

            if self.row & 0x04 != 0 {
                self.pins.a2.set_high().unwrap();
            } else {
                self.pins.a2.set_low().unwrap();
            }

            self.pins.oe.set_low().unwrap();
            delay.delay_us(100);
            self.pins.oe.set_high().unwrap();
        }
    }
}

mod text {
    pub struct Character<'a> {
        pub width: &'a usize,
        pub values: &'a [usize],
    }

    impl<'a> Character<'a> {
        const fn new(width: &'a usize, values: &'a [usize]) -> Self {
            Self { width, values }
        }
    }

    const CHARACTER_TABLE: [(char, Character); 40] = [
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

    pub const ICON_TABLE: [(&'static str, Icon); 17] = [
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
