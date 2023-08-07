#![no_std]
#![no_main]
#![feature(async_fn_in_trait)]
#![feature(type_alias_impl_trait)]
#![deny(missing_docs)]
#![forbid(clippy::missing_docs_in_private_items)]

//! Implementation of the Waveshare Pico Clock Green written in Rust.
//! This is evolving and not feature complete.

/// Use app module.
mod app;

/// Use button module.
mod buttons;

/// Use config module.
mod config;

/// Use clock module.
mod clock;

/// Use display module.
mod display;

/// Use pomodoro module.
mod pomodoro;

/// Use rtc module.
mod rtc;

/// Use temperature module.
mod temperature;

/// Use settings module.
mod settings;

/// Use speaker module.
mod speaker;

/// Use stopwatch module.
mod stopwatch;

use app::AppController;
use clock::ClockApp;
use display::{backlight::BacklightPins, display_matrix::DISPLAY_MATRIX, DisplayPins};
use ds323x::Ds323x;
use embassy_executor::{Executor, Spawner, _export::StaticCell};
use embassy_rp::{
    adc::{Adc, Channel, Config as ADCConfig, InterruptHandler},
    bind_interrupts,
    gpio::{Input, Level, Output, Pull},
    i2c::{self, Config as I2CConfig},
    multicore::Stack,
    peripherals::*,
};
use pomodoro::PomodoroApp;
use rtc::Ds3231;
use settings::SettingsApp;
use stopwatch::StopwatchApp;
use {defmt as _, defmt_rtt as _, panic_probe as _};

/// Executor for core 0.
static EXECUTOR0: StaticCell<Executor> = StaticCell::new();

/// Executor for core 1.
static EXECUTOR1: StaticCell<Executor> = StaticCell::new();

/// Preallocate stack memory for the second pico core.
static mut CORE1_STACK: Stack<4096> = Stack::new();

bind_interrupts!(struct Irqs {
    ADC_IRQ_FIFO => InterruptHandler;
});

/// Entry point.
#[cortex_m_rt::entry]
fn main() -> ! {
    let p = embassy_rp::init(Default::default());

    // init rtc
    let i2c = i2c::I2c::new_blocking(p.I2C1, p.PIN_7, p.PIN_6, I2CConfig::default());
    let ds323x: Ds323x<
        ds323x::interface::I2cInterface<i2c::I2c<'_, I2C1, i2c::Blocking>>,
        ds323x::ic::DS3231,
    > = Ds323x::new_ds3231(i2c);
    let ds3231 = Ds3231(ds323x);

    // init buttons
    let button_one: Input<'_, PIN_2> = Input::new(p.PIN_2, Pull::Up);
    let button_two: Input<'_, PIN_17> = Input::new(p.PIN_17, Pull::Up);
    let button_three: Input<'_, PIN_15> = Input::new(p.PIN_15, Pull::Up);

    // init speaker
    let speaker: Output<'_, PIN_14> = Output::new(p.PIN_14, Level::Low);

    // init display
    let a0: Output<'_, PIN_16> = Output::new(p.PIN_16, Level::Low);
    let a1: Output<'_, PIN_18> = Output::new(p.PIN_18, Level::Low);
    let a2: Output<'_, PIN_22> = Output::new(p.PIN_22, Level::Low);
    let oe: Output<'_, PIN_13> = Output::new(p.PIN_13, Level::Low);
    let sdi: Output<'_, PIN_11> = Output::new(p.PIN_11, Level::Low);
    let clk: Output<'_, PIN_10> = Output::new(p.PIN_10, Level::Low);
    let le: Output<'_, PIN_12> = Output::new(p.PIN_12, Level::Low);
    let adc = Adc::new(p.ADC, Irqs, ADCConfig::default());
    let ain = Channel::new_pin(p.PIN_26, Pull::None);
    let display_pins: DisplayPins<'_> = DisplayPins::new(a0, a1, a2, sdi, clk, le);
    let backlight_pins: BacklightPins<'_> = BacklightPins::new(oe, adc, ain);
    // let display: Display<'_> = Display::new(display_pins);

    embassy_rp::multicore::spawn_core1(p.CORE1, unsafe { &mut CORE1_STACK }, move || {
        let executor1 = EXECUTOR1.init(Executor::new());
        executor1.run(|spawner| {
            spawner
                .spawn(display_core(spawner, display_pins, backlight_pins))
                .unwrap()
        });
    });

    let executor0 = EXECUTOR0.init(Executor::new());
    executor0.run(|spawner| {
        spawner
            .spawn(main_core(
                spawner,
                ds3231,
                button_one,
                button_two,
                button_three,
                speaker,
            ))
            .unwrap();
    });
}

/// Task to run on the main core.
#[embassy_executor::task]
async fn main_core(
    spawner: Spawner,
    ds3231: Ds3231<'static>,
    button_one: Input<'static, PIN_2>,
    button_two: Input<'static, PIN_17>,
    button_three: Input<'static, PIN_15>,
    speaker: Output<'static, PIN_14>,
) {
    rtc::init(ds3231).await;

    spawner
        .spawn(display::display_matrix::process_text_buffer())
        .unwrap();

    spawner.spawn(buttons::button_one_task(button_one)).unwrap();
    spawner.spawn(buttons::button_two_task(button_two)).unwrap();
    spawner
        .spawn(buttons::button_three_task(button_three))
        .unwrap();

    spawner.spawn(speaker::speaker_task(speaker)).unwrap();

    let clock_app = ClockApp::new();
    let pomodoro_app = PomodoroApp::new();
    let stopwatch_app = StopwatchApp::new();
    let settings_app = SettingsApp::new();

    let mut app_controller = AppController::new(
        spawner,
        clock_app,
        pomodoro_app,
        stopwatch_app,
        settings_app,
    );
    app_controller.run_forever().await;
}

/// Task to run on the second core.
#[embassy_executor::task]
async fn display_core(
    spawner: Spawner,
    display_pins: DisplayPins<'static>,
    backlight_pins: BacklightPins<'static>,
) {
    spawner.spawn(display::update_matrix(display_pins)).unwrap();
    spawner
        .spawn(display::backlight::update_backlight(backlight_pins))
        .unwrap();

    let autolight_enabled = config::CONFIG.lock().await.borrow().get_autolight();
    DISPLAY_MATRIX.show_autolight_icon(autolight_enabled);
}
