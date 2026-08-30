use std::any::Any;
use std::time::{Duration, Instant};

pub trait Runtime {
    type JoinHandle<T>: Future<Output = Result<T, JoinError>>;
    type Sleep: Future<Output = ()>;
    type SleepUntil: Future<Output = ()>;
    type Timeout<F: Future>: Future<Output = Result<F::Output, TimeoutError>>;
    type TimeoutAt<F: Future>: Future<Output = Result<F::Output, TimeoutError>>;

    fn block_on<F>(&self, f: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send;

    fn spawn<F>(&self, f: F) -> Self::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static;

    fn shutdown(self, duration: Duration) -> Result<(), ShutdownError>;

    fn yield_now(&self) -> impl Future<Output = ()>;

    fn sleep(&self, duration: Duration) -> Self::Sleep;

    fn sleep_until(&self, time: Instant) -> Self::SleepUntil;

    fn timeout<F>(&self, duration: Duration, future: F) -> Self::Timeout<F>
    where
        F: Future;

    fn timeout_at<F>(&self, time: Instant, future: F) -> Self::TimeoutAt<F>
    where
        F: Future;
}

#[derive(Debug, thiserror::Error)]
pub enum JoinError {
    #[error("the task was cancelled")]
    Cancelled,
    #[error("the task panicked: {}", 0)]
    Panicked(Box<dyn Any + Send + 'static>),
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("timed out")]
pub struct TimeoutError;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ShutdownError {
    #[error("this is a proxy runtime, shut down the runtime through its primary handle")]
    ProxyRuntime,
}
