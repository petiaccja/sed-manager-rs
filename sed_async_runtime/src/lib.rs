mod cancellation_token;

use std::pin::Pin;
use std::task::Poll;
use std::time::{Duration, Instant};

pub use cancellation_token::{CancelSender, CancelToken, cancel_channel};

pub struct Runtime {
    inner: tokio::runtime::Runtime,
}

impl Runtime {
    pub fn multi_threaded() -> Self {
        Self {
            inner: tokio::runtime::Builder::new_multi_thread()
                .enable_all()
                .build()
                .expect("could not create tokio runtime"),
        }
    }

    pub fn block_on<F>(&self, f: F) -> F::Output
    where
        F: Future,
    {
        self.inner.block_on(f)
    }

    pub fn spawn<F>(&self, f: F) -> JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.inner.spawn(f).into()
    }

    pub fn shutdown(self, duration: Duration) {
        self.inner.shutdown_timeout(duration);
    }
}

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

pub async fn timeout<F>(duration: Duration, future: F) -> Result<F::Output, ()>
where
    F: Future,
{
    tokio::time::timeout(duration, future).await.map_err(|_| ())
}

pub async fn timeout_at<F>(time: Instant, future: F) -> Result<F::Output, ()>
where
    F: Future,
{
    tokio::time::timeout_at(time.into(), future).await.map_err(|_| ())
}
