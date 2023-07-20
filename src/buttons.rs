use embassy_rp::{gpio::Input, peripherals::*};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};
use embassy_time::{Duration, Instant, Timer};

pub enum ButtonPress {
    ShortPress,
    LongPress,
}

pub static BUTTON_ONE_PRESS: Signal<ThreadModeRawMutex, ButtonPress> = Signal::new();
pub static BUTTON_TWO_PRESS: Signal<ThreadModeRawMutex, ButtonPress> = Signal::new();
pub static BUTTON_THREE_PRESS: Signal<ThreadModeRawMutex, ButtonPress> = Signal::new();

#[embassy_executor::task]
pub async fn button_one_task(mut button: Input<'static, PIN_2>) -> ! {
    loop {
        button.wait_for_low().await;
        let start = Instant::now();
        button.wait_for_high().await;
        let end = Instant::now();

        let diff = end.duration_since(start).as_millis();

        if diff > 500 {
            BUTTON_ONE_PRESS.signal(ButtonPress::LongPress);
        } else {
            BUTTON_ONE_PRESS.signal(ButtonPress::ShortPress);
        }

        // add debounce
        Timer::after(Duration::from_millis(200)).await;
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
            BUTTON_TWO_PRESS.signal(ButtonPress::LongPress);
        } else {
            BUTTON_TWO_PRESS.signal(ButtonPress::ShortPress);
        }

        // add debounce
        Timer::after(Duration::from_millis(200)).await;
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
            BUTTON_THREE_PRESS.signal(ButtonPress::LongPress);
        } else {
            BUTTON_THREE_PRESS.signal(ButtonPress::ShortPress);
        }

        // add debounce
        Timer::after(Duration::from_millis(200)).await;
    }
}
