use embedded_hal::digital::v2::OutputPin;
use rp_pico::hal::gpio::{bank0::*, Output, Pin, PushPull};

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
    matrix: [[i32; 32]; 8],
    row: usize,
}

impl<'a> Display {
    pub fn new(pins: DisplayPins) -> Display {
        let mut matrix = [[0; 32]; 8];

        for (row_idx, row) in matrix.iter_mut().enumerate() {
            if row_idx > 0 {
                if row_idx % 2 == 0 {
                    for (col_idx, element) in row.iter_mut().enumerate() {
                        if col_idx < 2 || col_idx % 2 == 0 {
                            *element = 1;
                        }
                    }
                }
            } else {
                for element in row.iter_mut() {
                    *element = 1;
                }
            }
        }

        let dis = Self {
            pins,
            matrix,
            row: 0,
        };

        dis
    }

    pub fn update_display(&mut self) {
        self.row = (self.row + 1) % 8;

        for col in &self.matrix[self.row] {
            self.pins.clk.set_low().unwrap();
            if *col == 1 {
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
        self.pins.oe.set_high().unwrap();
    }
}
