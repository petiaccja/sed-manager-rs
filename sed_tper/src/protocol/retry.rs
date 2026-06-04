//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::time::Duration;
use std::time::Instant;

use sed_async::yield_now;

use crate::error::Error;

pub struct Retry {
    deadline: Instant,
    max_sleep_duration: Duration,
    current_sleep_duration: Duration,
}

impl Retry {
    pub fn new(deadline: Instant) -> Self {
        let start_time = Instant::now();
        let total_duration = std::cmp::max(deadline, start_time) - start_time;
        let max_sleep_duration = total_duration / 12;
        let initial_sleep_duration = std::cmp::min(total_duration / 200, Duration::from_micros(10));
        Self { deadline, max_sleep_duration, current_sleep_duration: initial_sleep_duration }
    }

    pub async fn sleep(&mut self) -> Result<(), Error> {
        let current_time = Instant::now();
        if self.deadline <= current_time {
            Err(Error::TimedOut)
        } else {
            let next_time = std::cmp::min(self.deadline, current_time + self.current_sleep_duration);
            sleep_until(next_time).await;
            self.current_sleep_duration = std::cmp::min(self.current_sleep_duration * 2, self.max_sleep_duration);
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
async fn sleep_until(time: Instant) {
    let current_time = Instant::now();
    let duration = std::cmp::max(time, current_time) - current_time;
    if duration < Duration::from_millis(1) {
        while Instant::now() < time {
            yield_now().await;
        }
    } else {
        sed_async::sleep_until(time).await;
    }
}
