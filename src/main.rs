#![no_std]
#![no_main]

mod display;
mod scheduler;

use crate::display::{Display, DisplayPins, DISPLAY_MATRIX};

use bsp::{
    entry,
    hal::multicore::{Multicore, Stack},
};
use cortex_m::delay::Delay;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::v2::InputPin;
use panic_probe as _;

use rp_pico as bsp;

use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    pac,
    sio::Sio,
    timer,
    watchdog::Watchdog,
};

/// Stack for core 1
///
/// Core 0 gets its stack via the normal route - any memory not used by static
/// values is reserved for stack and initialised by cortex-m-rt.
/// To get the same for Core 1, we would need to compile everything seperately
/// and modify the linker file for both programs, and that's quite annoying.
/// So instead, core1.spawn takes a [usize] which gets used for the stack.
/// NOTE: We use the `Stack` struct here to ensure that it has 32-byte
/// alignment, which allows the stack guard to take up the least amount of
/// usable RAM.
static mut CORE1_STACK: Stack<4096> = Stack::new();

#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let mut sio = Sio::new(pac.SIO);

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

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let timer = timer::Timer::new(pac.TIMER, &mut pac.RESETS);
    let mut scheduler = scheduler::Scheduler::new(timer);
    let show_led_schedule = scheduler::Schedule::new(show_led, true, "show_led", 500, 0);
    scheduler.add_schedule(show_led_schedule).unwrap();

    let button_one = pins.gpio2.into_pull_up_input();
    let button_two = pins.gpio17.into_pull_up_input();
    let button_three = pins.gpio15.into_pull_up_input();

    // init display
    let a0 = pins.gpio16.into_push_pull_output();
    let a1 = pins.gpio18.into_push_pull_output();
    let a2 = pins.gpio22.into_push_pull_output();
    let oe = pins.gpio13.into_push_pull_output();
    let sdi = pins.gpio11.into_push_pull_output();
    let clk = pins.gpio10.into_push_pull_output();
    let le = pins.gpio12.into_push_pull_output();
    let display_pins = DisplayPins::new(a0, a1, a2, oe, sdi, clk, le);
    let mut display = Display::new(display_pins);

    // Set up the delay for the first core.
    let sys_freq = clocks.system_clock.freq().to_Hz();

    let mut mc = Multicore::new(&mut pac.PSM, &mut pac.PPB, &mut sio.fifo);
    let cores = mc.cores();
    let core1 = &mut cores[1];

    core1
        .spawn(unsafe { &mut CORE1_STACK.mem }, move || {
            let core = unsafe { pac::CorePeripherals::steal() };
            let delay = Delay::new(core.SYST, sys_freq);
            display.update_display(delay);
        })
        .unwrap();
    loop {
        // scheduler.invoke_schedules();

        if button_one.is_low().unwrap() {
            DISPLAY_MATRIX.test_text();
            DISPLAY_MATRIX.test_icons();
            // speaker.set_high().unwrap();
        }

        if button_two.is_low().unwrap() {
            DISPLAY_MATRIX.clear();
        }

        if button_three.is_low().unwrap() {
            DISPLAY_MATRIX.fill();
        }
    }
}

fn show_led() {
    info!("Would show led!");
}

// End of file
