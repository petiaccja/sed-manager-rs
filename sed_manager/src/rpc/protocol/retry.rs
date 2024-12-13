use std::time::{Duration, Instant};

use crate::rpc::error::Error;

pub struct Retry {
    start_time: Instant,
    deadline: Instant,
    sleep_duration: Duration,
}

impl Retry {
    pub fn new(timeout: Duration) -> Self {
        let start_time = Instant::now();
        let deadline = start_time + timeout * 2;
        let sleep_duration = std::cmp::min(timeout / 200, Duration::from_micros(10));
        Self { start_time, deadline, sleep_duration }
    }

    pub async fn sleep(&mut self) -> Result<(), Error> {
        let current_time = Instant::now();
        if self.deadline <= current_time {
            Err(Error::TimedOut)
        } else {
            tokio::time::sleep(self.sleep_duration).await;
            self.sleep_duration = std::cmp::min(self.sleep_duration, (self.deadline - self.start_time) / 7);
            Ok(())
        }
    }
}
