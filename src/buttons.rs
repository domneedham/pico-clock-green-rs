use defmt::info;
use embassy_rp::{gpio::Input, peripherals::*};
use embassy_time::{Duration, Instant, Timer};

#[embassy_executor::task]
pub async fn button_one_task(mut button: Input<'static, PIN_2>) -> ! {
    loop {
        button.wait_for_low().await;
        let start = Instant::now();
        button.wait_for_high().await;
        let end = Instant::now();

        let diff = end.duration_since(start).as_millis();

        if diff > 500 {
            info!("Long press");
        } else {
            info!("Short press");
        }
    }
}

#[embassy_executor::task]
pub async fn button_two_task(mut button: Input<'static, PIN_17>) -> ! {
    loop {
        button.wait_for_low().await;
        let start = Instant::now();
        button.wait_for_high().await;
        let end = Instant::now();

        let diff = end.duration_since(start).as_millis();

        if diff > 500 {
            info!("Long press");
        } else {
            info!("Short press");
        }
    }
}

#[embassy_executor::task]
pub async fn button_three_task(mut button: Input<'static, PIN_15>) -> ! {
    loop {
        button.wait_for_low().await;
        let start = Instant::now();
        button.wait_for_high().await;
        let end = Instant::now();

        let diff = end.duration_since(start).as_millis();

        if diff > 500 {
            info!("Long press");
        } else {
            info!("Short press");
        }
    }
}
