use core::{borrow::BorrowMut, cell::RefCell};

use embassy_executor::Spawner;
use embassy_futures::select::{
    select,
    Either::{self},
};
use embassy_sync::{blocking_mutex::raw::ThreadModeRawMutex, mutex::Mutex, pubsub::PubSubChannel};
use embassy_time::{Duration, Timer};

use crate::{
    app::{App, StopAppTasks},
    buttons::ButtonPress,
    display::display_matrix::{TimeColon, DISPLAY_MATRIX},
    speaker::{self, SoundType},
};

/// Channel for firing events of when tasks should be stopped.
static STOP_APP_CHANNEL: PubSubChannel<ThreadModeRawMutex, StopAppTasks, 1, 1, 1> =
    PubSubChannel::new();

/// Depict the current running state of the stopwatch timer.
#[derive(Clone, Copy)]
enum RunningState {
    /// When the stopwatch app is first created or after reset. This should allow modification to the timer.
    NotStarted,

    /// When the countdown is running. This should *not* allow modification to the timer.
    Running,

    /// When the countdown has been paused. This should allow modification to the timer.
    Paused,

    /// When the countdown has finished (reached 00:00). This should *not* allow modification to the timer, reset instead.
    Finished,
}

/// Manage active state of the stopwatch app.
struct StopwatchState {
    /// The current running state.
    running: RunningState,

    /// The number of minutes to countdown from.
    minutes: u32,

    /// The number of seconds. Used for display purposes and should not be set during configuration.
    seconds: u32,
}

impl StopwatchState {
    /// Create a new stopwatch state with the set defaults.
    const fn new() -> Self {
        Self {
            running: RunningState::NotStarted,
            minutes: 0,
            seconds: 0,
        }
    }

    /// Reset the stopwatch state to the defaults it initialises with.
    pub fn reset(&mut self) {
        self.minutes = 0;
        self.seconds = 0;
        self.running = RunningState::NotStarted;
    }
}

/// Static reference to the pomo state so it can be accessed by static tasks.
static STOPWATCH_STATE: Mutex<ThreadModeRawMutex, RefCell<StopwatchState>> =
    Mutex::new(RefCell::new(StopwatchState::new()));

/// Stopwatch app.
/// Allows for setting starting a stopwatch upto 60 minutes.
pub struct StopwatchApp {}

impl StopwatchApp {
    /// Create a new stopwatch app.
    pub fn new() -> Self {
        Self {}
    }
}

impl App for StopwatchApp {
    fn get_name(&self) -> &str {
        "Stopwatch"
    }

    async fn start(&mut self, spawner: Spawner) {
        critical_section::with(|cs| {
            DISPLAY_MATRIX.clear_all(cs, true);
        });

        match get_running_state().await {
            RunningState::NotStarted => {}
            RunningState::Running => {}
            RunningState::Paused => spawner.spawn(stopwatch()).unwrap(),
            RunningState::Finished => STOPWATCH_STATE.lock().await.borrow_mut().get_mut().reset(),
        }

        show_time().await;
    }

    async fn stop(&mut self) {
        if let RunningState::Running = get_running_state().await {
            set_running(RunningState::Paused).await;
        }

        STOP_APP_CHANNEL
            .immediate_publisher()
            .publish_immediate(StopAppTasks);
    }

    async fn button_one_short_press(&mut self, spawner: Spawner) {
        match get_running_state().await {
            RunningState::NotStarted => {
                set_running(RunningState::Running).await;
                spawner.spawn(stopwatch()).unwrap()
            }
            RunningState::Running => {
                // due to running delay, 1s is lost on button press, so take them back away
                let (mut minutes, mut seconds) = get_time().await;

                if seconds == 59 {
                    minutes -= 1;
                    seconds = 0;
                } else {
                    seconds -= 1;
                }
                set_time(minutes, seconds).await;
                show_time().await;
                set_running(RunningState::Paused).await
            }
            RunningState::Paused => set_running(RunningState::Running).await,
            RunningState::Finished => {
                STOPWATCH_STATE.lock().await.borrow_mut().get_mut().reset();
                show_time().await;
            }
        }
    }

    async fn button_two_press(&mut self, press: ButtonPress, _: Spawner) {
        if let RunningState::Running = get_running_state().await {
            return;
        }

        let (mut minutes, mut seconds) = get_time().await;

        match press {
            ButtonPress::Long => {
                minutes = 0;
                seconds = 0;
            }
            ButtonPress::Short => {}
            ButtonPress::Double => {}
        }

        set_time(minutes, seconds).await;
        show_time().await;
    }

    async fn button_three_press(&mut self, press: ButtonPress, _: Spawner) {
        if let RunningState::Running = get_running_state().await {
            return;
        }

        let (mut minutes, mut seconds) = get_time().await;

        match press {
            ButtonPress::Long => {
                minutes = 0;
                seconds = 0;
            }
            ButtonPress::Short => {}
            ButtonPress::Double => {}
        }

        set_time(minutes, seconds).await;
        show_time().await;
    }
}

/// Get the running state value from the static stopwatch state.
async fn get_running_state() -> RunningState {
    STOPWATCH_STATE.lock().await.borrow().running
}

/// Get the (minutes, seconds) state value from the static stopwatch state.
async fn get_time() -> (u32, u32) {
    let minutes = STOPWATCH_STATE.lock().await.borrow().minutes;
    let seconds = STOPWATCH_STATE.lock().await.borrow().seconds;
    (minutes, seconds)
}

/// Set the new time to display and count down from on the static stopwatch state.
async fn set_time(minutes: u32, seconds: u32) {
    let mut guard = STOPWATCH_STATE.lock().await;
    let state = guard.borrow_mut().get_mut();

    state.minutes = minutes;
    state.seconds = seconds;
}

/// Set the running state on the static stopwatch state.
/// Will show/hide the CountDown icon on the display depending on the state passed.
async fn set_running(running: RunningState) {
    let mut guard = STOPWATCH_STATE.lock().await;
    let state = guard.borrow_mut().get_mut();

    state.running = running;

    if let RunningState::Running = running {
        DISPLAY_MATRIX.show_icon("CountUp");
    } else {
        DISPLAY_MATRIX.hide_icon("CountUp");
    }

    if let RunningState::Finished = running {
        speaker::sound(SoundType::RepeatLongBeep(3));
    }
}

/// Will show the time grabbed from the static stopwatch state.
async fn show_time() {
    let (minutes, seconds) = get_time().await;
    DISPLAY_MATRIX
        .queue_time(minutes, seconds, TimeColon::Full, 0, true, false)
        .await;
}

/// The stopwatch countdown loop.
///
/// Will continue to run as long as the running state is running or paused.
#[embassy_executor::task]
async fn stopwatch() {
    let mut stop_task_sub = STOP_APP_CHANNEL.subscriber().unwrap();

    show_time().await;

    loop {
        let running_state = get_running_state().await;
        match running_state {
            RunningState::NotStarted => break,
            RunningState::Running => {
                let (mut minutes, mut seconds) = get_time().await;
                show_time().await;

                if seconds == 59 {
                    if minutes == 59 {
                        set_running(RunningState::Finished).await;
                        break;
                    }

                    minutes += 1;

                    seconds = 0;
                } else {
                    seconds += 1
                }

                set_time(minutes, seconds).await;

                let res = select(
                    stop_task_sub.next_message(),
                    Timer::after(Duration::from_secs(1)),
                )
                .await;

                if let Either::First(_) = res {
                    break;
                }
            }
            RunningState::Paused => {
                Timer::after(Duration::from_millis(100)).await;
                continue;
            }
            RunningState::Finished => break,
        }
    }
}
