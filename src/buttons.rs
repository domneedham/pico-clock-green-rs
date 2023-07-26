use embassy_rp::{gpio::Input, peripherals::*};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};
use embassy_time::{Duration, Instant, Timer};

/// Type of button press made.
/// Double clicks are not yet supported but planned.
pub enum ButtonPress {
    /// When the button click duration is <=500ms.
    ShortPress,

    /// When the button click duration is >500ms.
    LongPress,
}

/// Signal for when the top button has been pressed.
pub static BUTTON_ONE_PRESS: Signal<ThreadModeRawMutex, ButtonPress> = Signal::new();

/// Signal for when the middle button has been pressed.
pub static BUTTON_TWO_PRESS: Signal<ThreadModeRawMutex, ButtonPress> = Signal::new();

/// Signal for when the bottom button has been pressed.
pub static BUTTON_THREE_PRESS: Signal<ThreadModeRawMutex, ButtonPress> = Signal::new();

/// Wait for changes async on the top button being pressed.
///
/// Will inform signal of button press after the full press has been completed.
/// The type of press is recorded in the ButtonPress enum.
///
/// This task has no way of cancellation.
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

/// Wait for changes async on the middle button being pressed.
///
/// Will inform signal of button press after the full press has been completed.
/// The type of press is recorded in the ButtonPress enum.
///
/// This task has no way of cancellation.
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

/// Wait for changes async on the bottom button being pressed.
///
/// Will inform signal of button press after the full press has been completed.
/// The type of press is recorded in the ButtonPress enum.
///
/// This task has no way of cancellation.
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
