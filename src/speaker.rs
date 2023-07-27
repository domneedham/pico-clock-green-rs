use embassy_rp::{gpio::Output, peripherals::*};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};

#[allow(dead_code)]

/// The type of sound the speaker should make.
pub enum SoundType {
    /// A short single beep.
    ShortBeep,

    /// A long single beep.
    LongBeep,

    /// A beep with a custom defined duration in milliseconds.
    Beep(u64),

    /// Repeat the short beep X times.
    RepeartShortBeep(u8),

    /// Repeat the long beep X times.
    RepeartLongBeep(u8),

    /// Repeat a custom duration beep X times.
    RepeatBeep(u8, u64),
}

/// Signal for when the speaker should sound.
static SOUND_SPEAKER: Signal<ThreadModeRawMutex, SoundType> = Signal::new();

/// Make the speaker play audio.
#[allow(dead_code)]
pub fn sound(t: SoundType) {
    SOUND_SPEAKER.signal(t);
}

/// Play audio on the speaker.
async fn play(speaker: &mut Output<'static, PIN_14>, times: u8, duration: Duration) {
    for _ in 0..times {
        speaker.set_high();
        Timer::after(duration).await;
        speaker.set_low();
        Timer::after(duration).await;
    }
}

/// Wait for a signal for the speaker to emit sound.
///
/// This task has no way of cancellation.
#[embassy_executor::task]
pub async fn speaker_task(mut speaker: Output<'static, PIN_14>) -> ! {
    loop {
        let sound_type = SOUND_SPEAKER.wait().await;

        match sound_type {
            SoundType::ShortBeep => play(&mut speaker, 1, Duration::from_millis(100)).await,
            SoundType::LongBeep => play(&mut speaker, 1, Duration::from_millis(500)).await,
            SoundType::Beep(duration) => {
                play(&mut speaker, 1, Duration::from_millis(duration)).await
            }
            SoundType::RepeartShortBeep(times) => {
                play(&mut speaker, times, Duration::from_millis(100)).await
            }
            SoundType::RepeartLongBeep(times) => {
                play(&mut speaker, times, Duration::from_millis(500)).await
            }
            SoundType::RepeatBeep(times, duration) => {
                play(&mut speaker, times, Duration::from_millis(duration)).await
            }
        }
    }
}
