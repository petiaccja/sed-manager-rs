mod cancellation_token;

use std::pin::Pin;
use std::task::Poll;
use std::time::{Duration, Instant};

pub use cancellation_token::{CancellationSender, CancellationToken, cancellation_channel};

pub struct JoinHandle<T> {
    inner: tokio::task::JoinHandle<T>,
}

impl<T> From<tokio::task::JoinHandle<T>> for JoinHandle<T> {
    fn from(value: tokio::task::JoinHandle<T>) -> Self {
        Self { inner: value }
    }
}

impl<T> Future for JoinHandle<T> {
    type Output = Result<T, ()>;

    fn poll(self: Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> Poll<Self::Output> {
        let inner = unsafe { Pin::map_unchecked_mut(self, |me| &mut me.inner) };
        match inner.poll(cx) {
            Poll::Ready(result) => Poll::Ready(result.map_err(|_err| ())),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    tokio::spawn(future).into()
}

pub async fn yield_now() {
    tokio::task::yield_now().await;
}

pub fn sleep(duration: Duration) -> impl Future<Output = ()> {
    tokio::time::sleep(duration)
}

pub fn sleep_until(time: Instant) -> impl Future<Output = ()> {
    tokio::time::sleep_until(time.into())
}
