use defmt::info;
use embassy_futures::select::{select, Either};
use embassy_rp::{gpio::Input, peripherals::*};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};

/// Type of button press made.
pub enum ButtonPress {
    /// When the button click duration is <=500ms.
    Short,

    /// When the button click duration is >500ms.
    Long,

    /// When the button click duration is <=500ms and a second click happens in the next 300ms.
    Double,
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
        // sit here until button is pressed down
        button.wait_for_low().await;

        let press = button_pressed(&mut button).await;
        BUTTON_ONE_PRESS.signal(press);

        // wait for button to be released
        if button.is_low() {
            button.wait_for_high().await;
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
        // sit here until button is pressed down
        button.wait_for_low().await;

        let press = button_pressed(&mut button).await;
        BUTTON_TWO_PRESS.signal(press);

        // wait for button to be released
        if button.is_low() {
            button.wait_for_high().await;
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
        // sit here until button is pressed down
        button.wait_for_low().await;

        let press = button_pressed(&mut button).await;
        BUTTON_THREE_PRESS.signal(press);

        // wait for button to be released
        if button.is_low() {
            button.wait_for_high().await;
        }

        // add debounce
        Timer::after(Duration::from_millis(200)).await;
    }
}

/// Determine the type of press performed on the button.
#[allow(clippy::needless_pass_by_ref_mut)] // needs to be mutable to use wait_for_*()
async fn button_pressed<T>(button: &mut Input<'_, T>) -> ButtonPress
where
    T: embassy_rp::gpio::Pin,
{
    // wait until button is released or 500ms (long press)
    let res = select(
        button.wait_for_high(),
        Timer::after(Duration::from_millis(500)),
    )
    .await;

    match res {
        // button is released before 500ms
        Either::First(_) => {
            // add debounce
            Timer::after(Duration::from_millis(50)).await;

            // see if button is pressed down again or 250ms
            let res = select(
                button.wait_for_low(),
                Timer::after(Duration::from_millis(250)),
            )
            .await;

            match res {
                // button is released before 250ms
                Either::First(_) => {
                    info!("Double press");
                    ButtonPress::Double
                }
                // 250ms passed by
                Either::Second(_) => {
                    info!("Short press");
                    ButtonPress::Short
                }
            }
        }
        // 500ms passed by
        Either::Second(_) => {
            info!("Long press");
            ButtonPress::Long
        }
    }
}
