use defmt::info;
use rp_pico::hal::{timer::Instant, Timer};

#[derive(Clone, Copy)]
pub struct Schedule<'a> {
    callback: fn(),
    enabled: bool,
    name: &'a str,
    duration: u64,
    initial_delay: u64,
    last_run_ticks: u64,
}

impl<'a> Default for Schedule<'a> {
    fn default() -> Self {
        Self {
            callback: Self::default_callback,
            enabled: Default::default(),
            name: Default::default(),
            duration: Default::default(),
            initial_delay: Default::default(),
            last_run_ticks: Default::default(),
        }
    }
}

impl<'a> Schedule<'a> {
    pub fn new(
        callback: fn(),
        enabled: bool,
        name: &'a str,
        duration: u64,
        initial_delay: u64,
    ) -> Schedule<'a> {
        Schedule {
            callback,
            enabled,
            name,
            duration,
            initial_delay,
            last_run_ticks: 0,
        }
    }

    pub fn invoke(&mut self, ticks: u64) {
        if self.enabled {
            (self.callback)();
            self.last_run_ticks = ticks;
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }

    pub fn default_callback() {
        info!("Non init callback");
    }
}

pub struct Scheduler<'a> {
    schedules: [Schedule<'a>; 10],
    count: usize,
    timer: Timer,
}

impl<'a> Scheduler<'a> {
    pub fn new(timer: Timer) -> Scheduler<'a> {
        Scheduler {
            schedules: [Schedule::default(); 10],
            count: 0,
            timer,
        }
    }

    pub fn add_schedule(&mut self, schedule: Schedule<'a>) -> Result<(), &'static str> {
        if self.count < 10 {
            self.schedules[self.count] = schedule;
            self.count += 1;
            Ok(())
        } else {
            Err("Callback list is full")
        }
    }

    pub fn invoke_schedules(&mut self) {
        for i in 0..self.count {
            let last_run = Instant::from_ticks(self.schedules[i].last_run_ticks);

            let current = self.timer.get_counter();

            let diff_ticks = current.checked_duration_since(last_run).unwrap().ticks() / 1000;

            if diff_ticks > self.schedules[i].duration {
                self.schedules[i].invoke(current.ticks());
            }
        }
    }
}
