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
    display::display_matrix::DISPLAY_MATRIX,
};

static STOP_APP_CHANNEL: PubSubChannel<ThreadModeRawMutex, StopAppTasks, 1, 1, 1> =
    PubSubChannel::new();

#[derive(Clone, Copy)]
enum RunningState {
    NotStarted,
    Running,
    Paused,
    Finished,
}

struct PomoState {
    running: RunningState,
    minutes: u32,
    seconds: u32,
}

impl PomoState {
    const fn new() -> Self {
        Self {
            running: RunningState::NotStarted,
            minutes: 30,
            seconds: 0,
        }
    }

    pub fn reset(&mut self) {
        self.minutes = 30;
        self.seconds = 0;
        self.running = RunningState::NotStarted;
    }
}

static POMO_STATE: Mutex<ThreadModeRawMutex, RefCell<PomoState>> =
    Mutex::new(RefCell::new(PomoState::new()));

pub struct PomodoroApp {}

impl PomodoroApp {
    pub fn new() -> Self {
        Self {}
    }
}

impl App for PomodoroApp {
    fn get_name(&self) -> &str {
        "Pomodoro"
    }

    async fn start(&mut self, _: Spawner) {
        critical_section::with(|cs| {
            DISPLAY_MATRIX.clear_all(cs, true);
        });

        if let RunningState::Finished = get_running_state().await {
            POMO_STATE.lock().await.borrow_mut().get_mut().reset();
        }

        show_time().await;
    }

    async fn stop(&mut self) {
        STOP_APP_CHANNEL
            .immediate_publisher()
            .publish_immediate(StopAppTasks());
    }

    async fn button_one_short_press(&mut self, spawner: Spawner) {
        match get_running_state().await {
            RunningState::NotStarted => spawner.spawn(countdown()).unwrap(),
            RunningState::Running => {
                // due to running delay, 1s is lost on button press, so add them back
                let (mut minutes, mut seconds) = get_time().await;

                if seconds == 59 {
                    minutes += 1;
                    seconds = 0;
                } else {
                    seconds += 1;
                }
                set_time(minutes, seconds).await;
                show_time().await;
                set_running(RunningState::Paused).await
            }
            RunningState::Paused => set_running(RunningState::Running).await,
            RunningState::Finished => spawner.spawn(countdown()).unwrap(),
        }
    }

    async fn button_two_press(&mut self, press: ButtonPress, _: Spawner) {
        if let RunningState::Running = get_running_state().await {
            return;
        }

        let (mut minutes, mut seconds) = get_time().await;

        match press {
            ButtonPress::LongPress => {
                minutes = 30;
                seconds = 0;
            }
            ButtonPress::ShortPress => {
                if minutes == 60 {
                    minutes = 1;
                } else {
                    minutes += 1;
                }
            }
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
            ButtonPress::LongPress => {
                minutes = 30;
                seconds = 0;
            }
            ButtonPress::ShortPress => {
                if minutes == 1 {
                    minutes = 60;
                } else {
                    minutes -= 1;
                }
            }
        }

        set_time(minutes, seconds).await;
        show_time().await;
    }
}

async fn get_running_state() -> RunningState {
    POMO_STATE.lock().await.borrow().running
}

async fn get_time() -> (u32, u32) {
    let minutes = POMO_STATE.lock().await.borrow().minutes;
    let seconds = POMO_STATE.lock().await.borrow().seconds;
    (minutes, seconds)
}

async fn set_time(minutes: u32, seconds: u32) {
    let mut guard = POMO_STATE.lock().await;
    let state = guard.borrow_mut().get_mut();

    state.minutes = minutes;
    state.seconds = seconds;
}

async fn set_running(running: RunningState) {
    let mut guard = POMO_STATE.lock().await;
    let state = guard.borrow_mut().get_mut();

    state.running = running;

    if let RunningState::Running = running {
        DISPLAY_MATRIX.show_icon("CountDown");
    } else {
        DISPLAY_MATRIX.hide_icon("CountDown");
    }
}

async fn show_time() {
    let (minutes, seconds) = get_time().await;
    DISPLAY_MATRIX.queue_time(minutes, seconds, 0, true).await;
}

#[embassy_executor::task]
async fn countdown() {
    let mut stop_task_sub = STOP_APP_CHANNEL.subscriber().unwrap();

    show_time().await;
    set_running(RunningState::Running).await;
    loop {
        let running_state = get_running_state().await;
        match running_state {
            RunningState::NotStarted => break,
            RunningState::Running => {
                let (mut minutes, mut seconds) = get_time().await;
                show_time().await;

                if seconds == 0 {
                    if minutes == 0 {
                        break;
                    }

                    minutes -= 1;

                    seconds = 59;
                } else {
                    seconds -= 1;
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
    set_running(RunningState::Finished).await;
}
