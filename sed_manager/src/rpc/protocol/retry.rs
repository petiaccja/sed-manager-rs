use std::time::{Duration, Instant};

use crate::rpc::error::{Error, ErrorEvent};

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

    pub fn sleep(&mut self) -> Result<(), Error> {
        let current_time = Instant::now();
        if self.deadline <= current_time {
            Err(ErrorEvent::TimedOut.into())
        } else {
            sleep(self.sleep_duration);
            self.sleep_duration = std::cmp::min(self.sleep_duration, (self.deadline - self.start_time) / 7);
            Ok(())
        }
    }
}

/// Combines busy wait and OS sleep to introduce delay.
///
/// For short sleeps of just a few milliseconds, the OS sleep function might
/// sleep far longer than we need. This would mean that the device is polled
/// with IF-RECV much less often, and that can slow down the synchronous
/// communication protocol by a large margin. A finer loop-based sleep should
/// allow the device to be polled for replies often.
fn sleep(duration: Duration) {
    if duration < Duration::from_millis(8) {
        let start = Instant::now();
        let end = start + duration;
        while Instant::now() < end {
            std::thread::yield_now();
        }
    } else {
        std::thread::sleep(duration);
    }
}
