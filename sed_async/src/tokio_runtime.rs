use std::{
    pin::Pin,
    task::{Context, Poll},
};

use pin_project::pin_project;

use crate::{
    Runtime,
    runtime::{JoinError, ShutdownError, TimeoutError},
};

#[derive(Debug)]
pub enum TokioRuntime {
    Runtime(tokio::runtime::Runtime),
    Handle(tokio::runtime::Handle),
}

impl TokioRuntime {
    pub fn multi_threaded(num_threads: Option<usize>) -> Result<TokioRuntime, std::io::Error> {
        let mut builder = tokio::runtime::Builder::new_multi_thread();
        builder.enable_time();
        if let Some(num_threads) = num_threads {
            let _ = builder.worker_threads(num_threads);
        }
        builder.build().map(|runtime| TokioRuntime::Runtime(runtime))
    }

    pub fn current() -> Option<TokioRuntime> {
        tokio::runtime::Handle::try_current().ok().map(|handle| TokioRuntime::Handle(handle))
    }

    fn handle(&self) -> &tokio::runtime::Handle {
        match self {
            TokioRuntime::Runtime(runtime) => runtime.handle(),
            TokioRuntime::Handle(handle) => handle,
        }
    }
}

impl Runtime for TokioRuntime {
    type JoinHandle<T> = TokioJoinHandle<T>;
    type Sleep = TokioSleep;
    type SleepUntil = TokioSleep;
    type Timeout<F: Future> = TokioTimeout<F>;
    type TimeoutAt<F: Future> = TokioTimeout<F>;

    fn block_on<F>(&self, f: F) -> F::Output
    where
        F: Future,
    {
        self.handle().block_on(f)
    }

    fn spawn<F>(&self, f: F) -> Self::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        TokioJoinHandle { inner: self.handle().spawn(f) }
    }

    /// Shuts down the runtime if the underlying [`tokio`] runtime is owned
    /// by `self`. If the underlying runtime is just a [`Handle`],
    /// [`ShutdownError::ProxyRuntime`] is returned.
    ///
    /// [`Handle`]: tokio::runtime::Handle
    fn shutdown(self, duration: std::time::Duration) -> Result<(), ShutdownError> {
        match self {
            TokioRuntime::Runtime(runtime) => {
                runtime.shutdown_timeout(duration);
                Ok(())
            }
            TokioRuntime::Handle(_handle) => Err(ShutdownError::ProxyRuntime),
        }
    }

    fn yield_now(&self) -> impl Future<Output = ()> {
        tokio::task::yield_now()
    }

    fn sleep(&self, duration: std::time::Duration) -> Self::Sleep {
        tokio::time::sleep(duration)
    }

    fn sleep_until(&self, time: std::time::Instant) -> Self::SleepUntil {
        tokio::time::sleep_until(time.into())
    }

    fn timeout<F>(&self, duration: std::time::Duration, future: F) -> Self::Timeout<F>
    where
        F: Future,
    {
        TokioTimeout { inner: tokio::time::timeout(duration, future) }
    }

    fn timeout_at<F>(&self, time: std::time::Instant, future: F) -> Self::TimeoutAt<F>
    where
        F: Future,
    {
        TokioTimeout { inner: tokio::time::timeout_at(time.into(), future) }
    }
}

#[derive(Debug)]
#[pin_project]
pub struct TokioJoinHandle<T> {
    #[pin]
    inner: tokio::task::JoinHandle<T>,
}

impl<T> Future for TokioJoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().inner.poll(cx) {
            Poll::Ready(result) => Poll::Ready(result.map_err(|err| {
                if let Ok(panic) = err.try_into_panic() {
                    JoinError::Panicked(panic)
                } else {
                    JoinError::Cancelled
                }
            })),
            Poll::Pending => Poll::Pending,
        }
    }
}

pub type TokioSleep = tokio::time::Sleep;

#[derive(Debug)]
#[pin_project]
pub struct TokioTimeout<F> {
    #[pin]
    inner: tokio::time::Timeout<F>,
}

impl<F> Future for TokioTimeout<F>
where
    F: Future,
{
    type Output = Result<F::Output, TimeoutError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().inner.poll(cx) {
            Poll::Ready(result) => Poll::Ready(result.map_err(|_err| TimeoutError)),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::{
        sync::Arc,
        time::{Duration, Instant},
    };

    use googletest::{assert_that, matchers::*};

    #[test]
    fn spawn() {
        let runtime = TokioRuntime::multi_threaded(Some(1)).unwrap();
        let (tx, rx) = oneshot::channel();
        runtime.spawn(async {
            let _ = tx.send(());
        });
        assert_that!(rx.recv_timeout(Duration::from_secs(5)), ok(anything()));
    }

    #[test]
    fn sleep() {
        let runtime = Arc::new(TokioRuntime::multi_threaded(Some(1)).unwrap());
        let runtime_ = runtime.clone();
        let (tx, rx) = oneshot::channel();
        runtime.spawn(async move {
            let start = Instant::now();
            let _ = runtime_.sleep(Duration::from_millis(16)).await;
            let end = Instant::now();
            let _ = tx.send(end - start);
        });
        assert_that!(rx.recv_timeout(Duration::from_secs(5)), ok(gt(Duration::from_millis(15))));
    }

    #[test]
    fn sleep_until() {
        let runtime = Arc::new(TokioRuntime::multi_threaded(Some(1)).unwrap());
        let runtime_ = runtime.clone();
        let (tx, rx) = oneshot::channel();
        runtime.spawn(async move {
            let start = Instant::now();
            let _ = runtime_.sleep_until(start + Duration::from_millis(16)).await;
            let end = Instant::now();
            let _ = tx.send(end - start);
        });
        assert_that!(rx.recv_timeout(Duration::from_secs(5)), ok(gt(Duration::from_millis(15))));
    }

    #[test]
    fn timeout_completed() {
        let runtime = Arc::new(TokioRuntime::multi_threaded(Some(1)).unwrap());
        let runtime_ = runtime.clone();
        let (tx, rx) = oneshot::channel();
        runtime.spawn(async move {
            let result = runtime_.timeout(Duration::from_millis(16), async {}).await;
            let _ = tx.send(result);
        });
        assert_that!(rx.recv_timeout(Duration::from_secs(5)), ok(ok(eq(&()))));
    }

    #[test]
    fn timeout_failed() {
        let runtime = Arc::new(TokioRuntime::multi_threaded(Some(1)).unwrap());
        let runtime_ = runtime.clone();
        let (tx, rx) = oneshot::channel();
        runtime.spawn(async move {
            let result = runtime_
                .timeout(Duration::from_millis(16), async {
                    std::future::pending::<()>().await;
                })
                .await;
            let _ = tx.send(result);
        });
        assert_that!(rx.recv_timeout(Duration::from_secs(5)), ok(err(eq(&TimeoutError))));
    }

    #[test]
    fn timeout_at_completed() {
        let runtime = Arc::new(TokioRuntime::multi_threaded(Some(1)).unwrap());
        let runtime_ = runtime.clone();
        let (tx, rx) = oneshot::channel();
        runtime.spawn(async move {
            let result = runtime_.timeout_at(Instant::now() + Duration::from_millis(16), async {}).await;
            let _ = tx.send(result);
        });
        assert_that!(rx.recv_timeout(Duration::from_secs(5)), ok(ok(eq(&()))));
    }

    #[test]
    fn timeout_at_failed() {
        let runtime = Arc::new(TokioRuntime::multi_threaded(Some(1)).unwrap());
        let runtime_ = runtime.clone();
        let (tx, rx) = oneshot::channel();
        runtime.spawn(async move {
            let result = runtime_
                .timeout_at(Instant::now() + Duration::from_millis(16), async {
                    std::future::pending::<()>().await;
                })
                .await;
            let _ = tx.send(result);
        });
        assert_that!(rx.recv_timeout(Duration::from_secs(5)), ok(err(eq(&TimeoutError))));
    }
}
