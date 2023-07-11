#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

mod display;

use crate::display::{Display, DisplayPins};

use display::display_matrix::DISPLAY_MATRIX;
use embassy_executor::{Executor, Spawner, _export::StaticCell};
use embassy_rp::{
    gpio::{Input, Level, Output, Pull},
    multicore::Stack,
    peripherals::*,
};
use embassy_time::{Duration, Timer};
use {defmt as _, defmt_rtt as _, panic_probe as _};

static EXECUTOR0: StaticCell<Executor> = StaticCell::new();
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();
static mut CORE1_STACK: Stack<4096> = Stack::new();

#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    // init buttons
    let button_one: Input<'_, PIN_2> = Input::new(p.PIN_2, Pull::Up);
    let button_two: Input<'_, PIN_17> = Input::new(p.PIN_17, Pull::Up);
    let button_three: Input<'_, PIN_15> = Input::new(p.PIN_15, Pull::Up);

    // init display
    let a0: Output<'_, PIN_16> = Output::new(p.PIN_16, Level::Low);
    let a1: Output<'_, PIN_18> = Output::new(p.PIN_18, Level::Low);
    let a2: Output<'_, PIN_22> = Output::new(p.PIN_22, Level::Low);
    let oe: Output<'_, PIN_13> = Output::new(p.PIN_13, Level::Low);
    let sdi: Output<'_, PIN_11> = Output::new(p.PIN_11, Level::Low);
    let clk: Output<'_, PIN_10> = Output::new(p.PIN_10, Level::Low);
    let le: Output<'_, PIN_12> = Output::new(p.PIN_12, Level::Low);
    let display_pins: DisplayPins<'_> = DisplayPins::new(a0, a1, a2, oe, sdi, clk, le);
    let display: Display<'_> = Display::new(display_pins);

    embassy_rp::multicore::spawn_core1(p.CORE1, unsafe { &mut CORE1_STACK }, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        executor1.run(|spawner| spawner.spawn(display_core(display)).unwrap());
    });

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        spawner
            .spawn(main_core(spawner, button_one, button_two, button_three))
            .unwrap();
    });
}

#[embassy_executor::task]
async fn main_core(
    spawner: Spawner,
    button_one: Input<'static, PIN_2>,
    button_two: Input<'static, PIN_17>,
    button_three: Input<'static, PIN_15>,
) -> ! {
    spawner
        .spawn(display::display_matrix::process_text_buffer())
        .unwrap();

    loop {
        if button_one.is_low() {
            DISPLAY_MATRIX.test_text().await;

            critical_section::with(|cs| {
                DISPLAY_MATRIX.test_icons(cs);
            });
        }

        if button_two.is_low() {
            critical_section::with(|cs| {
                DISPLAY_MATRIX.clear(cs);
            });
        }

        if button_three.is_low() {
            critical_section::with(|cs| {
                DISPLAY_MATRIX.fill_all(cs);
            });
        }

        Timer::after(Duration::from_millis(100)).await;
    }
}

#[embassy_executor::task]
async fn display_core(mut display: Display<'static>) -> ! {
    loop {
        display.update_display().await;
    }
}
