#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

mod display;

use crate::display::{Display, DisplayPins, DISPLAY_MATRIX};

use embassy_executor::{Executor, _export::StaticCell};
use embassy_rp::{
    gpio::{Input, Level, Output, Pull},
    multicore::Stack,
};
use {defmt as _, defmt_rtt as _, panic_probe as _};

static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static mut CORE1_STACK: Stack<4096> = Stack::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    // Initialise Peripherals
    let p = embassy_rp::init(Default::default());

    let button_one = Input::new(p.PIN_2, Pull::Up);
    let button_two = Input::new(p.PIN_17, Pull::Up);
    let button_three = Input::new(p.PIN_15, Pull::Up);

    // init display
    let a0: Output<'_, embassy_rp::peripherals::PIN_16> = Output::new(p.PIN_16, Level::Low);
    let a1 = Output::new(p.PIN_18, Level::Low);
    let a2 = Output::new(p.PIN_22, Level::Low);
    let oe = Output::new(p.PIN_13, Level::Low);
    let sdi = Output::new(p.PIN_11, Level::Low);
    let clk = Output::new(p.PIN_10, Level::Low);
    let le = Output::new(p.PIN_12, Level::Low);
    let display_pins = DisplayPins::new(a0, a1, a2, oe, sdi, clk, le);
    let display = Display::new(display_pins);

    embassy_rp::multicore::spawn_core1(p.CORE1, unsafe { &mut CORE1_STACK }, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        executor1.run(|spawner| spawner.spawn(display_core(display)).unwrap());
    });

    loop {
        // scheduler.invoke_schedules();

        critical_section::with(|cs| {
            if button_one.is_low() {
                DISPLAY_MATRIX.test_text(cs);
                DISPLAY_MATRIX.test_icons(cs);
                // speaker.set_high().unwrap();
            }

            if button_two.is_low() {
                DISPLAY_MATRIX.clear(cs);
            }

            if button_three.is_low() {
                DISPLAY_MATRIX.fill(cs);
            }
        })
    }
}

#[embassy_executor::task]
async fn display_core(mut display: Display<'static>) {
    display.update_display().await;
}
