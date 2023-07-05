#![no_std]
#![no_main]

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

    let mut speaker = pins.gpio14.into_push_pull_output();
    let button_one = pins.gpio2.into_pull_up_input();
    // let button_two = pins.gpio17.into_pull_up_input();
    // let button_three = pins.gpio15.into_pull_up_input();

    let mut a0 = pins.gpio16.into_push_pull_output();
    let mut a1 = pins.gpio18.into_push_pull_output();
    let mut a2 = pins.gpio22.into_push_pull_output();
    let mut oe = pins.gpio13.into_push_pull_output();
    let mut sdi = pins.gpio11.into_push_pull_output();
    let mut clk = pins.gpio10.into_push_pull_output();
    let mut le = pins.gpio12.into_push_pull_output();

    let mut matrix: [[i32; 32]; 8] = [[0; 32]; 8];
    let mut row = 0;

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

    loop {
        if button_one.is_low().unwrap() {
            speaker.set_high().unwrap();
        } else {
            speaker.set_low().unwrap();
        }

        row = (row + 1) % 8;

        for col in &matrix[row] {
            clk.set_low().unwrap();
            if *col == 1 {
                sdi.set_high().unwrap();
            } else {
                sdi.set_low().unwrap();
            }
            clk.set_high().unwrap();
        }

        le.set_high().unwrap();
        le.set_low().unwrap();

        if row & 0x01 != 0 {
            a0.set_high().unwrap();
        } else {
            a0.set_low().unwrap();
        }

        if row & 0x02 != 0 {
            a1.set_high().unwrap();
        } else {
            a1.set_low().unwrap();
        }

        if row & 0x04 != 0 {
            a2.set_high().unwrap();
        } else {
            a2.set_low().unwrap();
        }

        oe.set_low().unwrap();
        delay.delay_us(100);
        oe.set_high().unwrap();
    }
}

// End of file
