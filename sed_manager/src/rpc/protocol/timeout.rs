use std::{collections::VecDeque, time::Instant};

use crate::rpc::{Error, Properties};

pub struct Timeout<T> {
    properties: Properties,
    buffer: VecDeque<Result<T, Error>>,
    last_method_time: Instant,
    error: Option<Error>,
}

impl<T> Timeout<T> {
    pub fn new(properties: Properties) -> Self {
        Self { properties, buffer: VecDeque::new(), last_method_time: Instant::now(), error: None }
    }

    pub fn enqueue(&mut self, method: Result<T, Error>) {
        self.last_method_time = Instant::now();
        self.buffer.push_back(method);
    }

    pub fn poll(&mut self) -> Option<Result<T, Error>> {
        let elapsed = Instant::now() - self.last_method_time;

        if let Some(error) = &self.error {
            self.buffer.clear();
            Some(Err(error.clone()))
        } else if let Some(response) = self.buffer.pop_front() {
            Some(response)
        } else if elapsed > self.properties.trans_timeout {
            let response = Err(Error::TimedOut);
            self.error = Some(Error::AbortedByHost);
            Some(response)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::*;

    #[test]
    fn timeout_empty() {
        let mut timeout =
            Timeout::<i32>::new(Properties { trans_timeout: Duration::from_secs(1000), ..Default::default() });
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(timeout.poll(), None);
    }

    #[test]
    fn timeout_timed_out() {
        let mut timeout =
            Timeout::<i32>::new(Properties { trans_timeout: Duration::from_millis(0), ..Default::default() });
        std::thread::sleep(Duration::from_millis(50));
        assert_eq!(timeout.poll(), Some(Err(Error::TimedOut)));
        assert_eq!(timeout.poll(), Some(Err(Error::AbortedByHost))); // Error should be repeated.
    }

    #[test]
    fn timeout_some() {
        let mut timeout = Timeout::new(Properties { trans_timeout: Duration::from_secs(1000), ..Default::default() });
        timeout.enqueue(Ok(0i32));
        assert_eq!(timeout.poll(), Some(Ok(0)));
    }
}
