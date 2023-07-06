pub struct Schedule<'a> {
    callback: fn(),
    enabled: bool,
    name: &'a str,
    duration: u32,
    initial_delay: u32,
}

impl<'a> Schedule<'a> {
    pub fn new(
        callback: fn(),
        enabled: bool,
        name: &'a str,
        duration: u32,
        initial_delay: u32,
    ) -> Schedule<'a> {
        Schedule {
            callback,
            enabled,
            name,
            duration,
            initial_delay,
        }
    }

    pub fn invoke(&self) {
        if self.enabled {
            (self.callback)();
        }
    }

    pub fn enable(&mut self) {
        self.enabled = true;
    }

    pub fn disable(&mut self) {
        self.enabled = false;
    }
}

pub struct Scheduler<'a> {
    schedules: [Option<&'a Schedule<'a>>; 10],
    count: usize,
}

impl<'a> Scheduler<'a> {
    pub fn new() -> Scheduler<'a> {
        Scheduler {
            schedules: [None; 10],
            count: 0,
        }
    }

    pub fn add_schedule(&mut self, schedule: &'a Schedule<'a>) -> Result<(), &'static str> {
        if self.count < 10 {
            self.schedules[self.count] = Some(schedule);
            self.count += 1;
            Ok(())
        } else {
            Err("Callback list is full")
        }
    }

    pub fn remove_schedule(&mut self, schedule: &'a Schedule<'a>) {
        let mut i = 0;
        while i < self.count {
            if let Some(existing_callback) = self.schedules[i] {
                if existing_callback as *const _ == schedule as *const _ {
                    // Shift remaining elements to the left
                    for j in i..self.count - 1 {
                        self.schedules[j] = self.schedules[j + 1];
                    }
                    self.count -= 1;
                    break;
                }
            }
            i += 1;
        }
    }

    pub fn invoke_schedules(&self) {
        for i in 0..self.count {
            if let Some(schedule) = self.schedules[i] {
                schedule.invoke();
            }
        }
    }
}
