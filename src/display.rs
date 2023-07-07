use core::{cell::UnsafeCell, u8};

use cortex_m::delay::Delay;
use embedded_hal::digital::v2::OutputPin;
use rp_pico::hal::gpio::{bank0::*, Output, Pin, PushPull};

struct DisplaySet<'a> {
    ziku: [Character<'a>; 1],
    icons: [Icon<'a>; 1],
}

impl<'a> DisplaySet<'a> {
    const fn new() -> DisplaySet<'a> {
        Self {
            ziku: Self::build_ziku(),
            icons: Self::build_icons(),
        }
    }

    const fn build_ziku() -> [Character<'a>; 1] {
        [Character::new(
            ":",
            &2,
            &[0x00, 0x03, 0x03, 0x00, 0x03, 0x03, 0x00],
        )]
    }

    const fn build_icons() -> [Icon<'a>; 1] {
        [Icon::new("MoveOn", 0, 0, 2)]
    }
}

pub struct DisplayMatrix<'a>((UnsafeCell<[[i32; 32]; 8]>, DisplaySet<'a>));

impl<'a> DisplayMatrix<'_> {
    pub fn get_matrix(&self) -> &[[i32; 32]; 8] {
        unsafe { self.0 .0.get().as_ref().unwrap() }
    }

    pub fn test_leds(&self) {
        unsafe { *self.0 .0.get() = [[1; 32]; 8] };
    }

    pub fn clear(&self) {
        unsafe { *self.0 .0.get() = [[0; 32]; 8] };
    }
}

unsafe impl Sync for DisplayMatrix<'_> {}

const DISPLAY_MATRIX_INIT: DisplayMatrix =
    DisplayMatrix((UnsafeCell::new([[1; 32]; 8]), DisplaySet::new()));

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

            for col in DISPLAY_MATRIX.get_matrix()[self.row] {
                self.pins.clk.set_low().unwrap();
                if col == 1 {
                    self.pins.sdi.set_high().unwrap();
                } else {
                    self.pins.sdi.set_low().unwrap();
                }
                self.pins.clk.set_high().unwrap();
            }

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

struct Character<'a> {
    name: &'a str,
    width: &'a u8,
    values: &'a [u8],
}

impl<'a> Character<'a> {
    const fn new(name: &'a str, width: &'a u8, values: &'a [u8]) -> Self {
        Self {
            name,
            width,
            values,
        }
    }
}

struct Icon<'a> {
    name: &'a str,
    x: u8,
    y: u8,
    width: u8,
}

impl<'a> Icon<'a> {
    const fn new(name: &'a str, x: u8, y: u8, width: u8) -> Icon {
        Self { name, x, y, width }
    }
}
