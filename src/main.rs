#![no_std]
#![no_main]

mod display;
mod scheduler;

use bsp::entry;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    watchdog::Watchdog,
};

use crate::{
    display::{Display, DisplayPins},
    scheduler::Schedule,
};

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut scheduler = scheduler::Scheduler::new();
    let show_led_schedule = scheduler::Schedule::new(show_led, true, "show_led", 100, 0);
    scheduler.add_schedule(&show_led_schedule).unwrap();

    let mut speaker = pins.gpio14.into_push_pull_output();
    let button_one = pins.gpio2.into_pull_up_input();
    // let button_two = pins.gpio17.into_pull_up_input();
    // let button_three = pins.gpio15.into_pull_up_input();

    let a0 = pins.gpio16.into_push_pull_output();
    let a1 = pins.gpio18.into_push_pull_output();
    let a2 = pins.gpio22.into_push_pull_output();
    let oe = pins.gpio13.into_push_pull_output();
    let sdi = pins.gpio11.into_push_pull_output();
    let clk = pins.gpio10.into_push_pull_output();
    let le = pins.gpio12.into_push_pull_output();

    let display_pins = DisplayPins::new(a0, a1, a2, oe, sdi, clk, le);
    let mut display = Display::new(display_pins);

    loop {
        scheduler.invoke_schedules();

        if button_one.is_low().unwrap() {
            speaker.set_high().unwrap();
        } else {
            speaker.set_low().unwrap();
        }

        display.update_display();
    }
}

fn show_led() {
    info!("Would show led!");
}

// End of file
