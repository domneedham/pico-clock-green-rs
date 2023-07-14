use defmt::info;
use embassy_executor::Spawner;
use embassy_time::{Duration, Timer};

use crate::{app::App, display::display_matrix::DISPLAY_MATRIX};

#[derive(PartialEq)]
pub struct ClockApp<'a> {
    pub name: &'a str,
}

impl<'a> App<'a> for ClockApp<'a> {
    fn get_name(&self) -> &'a str {
        self.name
    }

    async fn start(&self, spawner: Spawner) {
        DISPLAY_MATRIX.queue_text("21:21", true).await;
        spawner.spawn(clock()).unwrap();
    }

    async fn stop(&self) {
        // do nothing yet.
    }
}

#[embassy_executor::task]
async fn clock() -> ! {
    loop {
        info!("Would update time");
        Timer::after(Duration::from_secs(1)).await;
    }
}
