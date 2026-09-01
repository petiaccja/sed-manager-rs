use std::{
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use pin_project::pin_project;
use slint::EventLoopError;

use crate::{
    Runtime,
    runtime::{JoinError, ShutdownError, TimeoutError},
};

#[derive(Debug)]
pub struct SlintRuntime;

impl Runtime for SlintRuntime {
    type JoinHandle<T> = SlintJoinHandle<T>;
    type Sleep = SlintSleep;
    type SleepUntil = SlintSleep;
    type Timeout<F: Future> = SlintTimeout<F>;
    type TimeoutAt<F: Future> = SlintTimeout<F>;

    // Spawns `f` on the runtime, blocks the current thread until it finishies,
    // and returns the result of the spawned future.
    //
    // DO NOT EVER CALL THIS FROM THE SLINT EVENT LOOP! It will deadlock your
    // application.
    fn block_on<F>(&self, f: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send,
    {
        let (tx, rx) = oneshot::channel();
        self.spawn(async move {
            let _ = tx.send(f.await);
        });
        match rx.recv() {
            Ok(value) => value,
            Err(_) => panic!("the event loop was terminated while the future was running"),
        }
    }

    fn spawn<F>(&self, f: F) -> Self::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        let (spawn_tx, spawn_rx) = oneshot::async_channel();
        let invoke_result = slint::invoke_from_event_loop(move || {
            let _ = spawn_tx.send(slint::spawn_local(f));
        });
        match invoke_result {
            Ok(_) => SlintJoinHandle::Spawn(spawn_rx),
            Err(error) => panic!("the event loop is not running: {error}"),
        }
    }

    /// Returns [`ShutdownError::ProxyRuntime`] and does not modify the runtime.
    fn shutdown(self, _duration: Duration) -> Result<(), ShutdownError> {
        Err(ShutdownError::ProxyRuntime)
    }

    fn yield_now(&self) -> impl Future<Output = ()> {
        async {
            // Slint does not seem to have a yield equivalent.
        }
    }

    fn sleep(&self, duration: Duration) -> Self::Sleep {
        let (tx, rx) = oneshot::async_channel();
        if duration == Duration::ZERO {
            let _ = tx.send(());
        } else {
            let invoke_result = slint::invoke_from_event_loop(move || {
                slint::Timer::single_shot(duration, move || {
                    let _ = tx.send(());
                })
            });
            match invoke_result {
                Ok(_) => (),
                Err(error) => panic!("the event loop is not running: {error}"),
            };
        }
        SlintSleep { elapsed: rx }
    }

    fn sleep_until(&self, time: Instant) -> Self::SleepUntil {
        let duration = time.saturating_duration_since(Instant::now());
        self.sleep(duration)
    }

    fn timeout<F>(&self, duration: Duration, future: F) -> Self::Timeout<F>
    where
        F: Future,
    {
        let (tx, rx) = oneshot::async_channel();
        let invoke_result = slint::invoke_from_event_loop(move || {
            slint::Timer::single_shot(duration, move || {
                let _ = tx.send(());
            })
        });
        match invoke_result {
            Ok(_) => (),
            Err(error) => panic!("the event loop is not running: {error}"),
        };
        SlintTimeout { timeout: rx, future }
    }

    fn timeout_at<F>(&self, time: Instant, future: F) -> Self::TimeoutAt<F>
    where
        F: Future,
    {
        let duration = time.saturating_duration_since(Instant::now());
        self.timeout(duration, future)
    }
}

#[pin_project(project = SlintJoinHandleProj)]
pub enum SlintJoinHandle<T> {
    Spawn(#[pin] oneshot::AsyncReceiver<Result<slint::JoinHandle<T>, EventLoopError>>),
    Join(#[pin] slint::JoinHandle<T>),
}

impl<T> Future for SlintJoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.as_mut().project() {
            SlintJoinHandleProj::Spawn(receiver) => match receiver.poll(cx) {
                Poll::Ready(Ok(Ok(join_handle))) => {
                    *self = SlintJoinHandle::Join(join_handle);
                    cx.waker().wake_by_ref();
                    Poll::Pending
                }
                Poll::Ready(Ok(Err(_))) | Poll::Ready(Err(_)) => Poll::Ready(Err(JoinError::Cancelled)),
                Poll::Pending => Poll::Pending,
            },
            SlintJoinHandleProj::Join(join_handle) => match join_handle.poll(cx) {
                Poll::Ready(value) => Poll::Ready(Ok(value)),
                Poll::Pending => Poll::Pending,
            },
        }
    }
}

impl<T> core::fmt::Debug for SlintJoinHandle<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Spawn(_) => write!(f, "SlintJoinHandle::Spawn"),
            Self::Join(_) => write!(f, "SlintJoinHandle::Join"),
        }
    }
}

#[derive(Debug)]
#[pin_project]
pub struct SlintSleep {
    #[pin]
    elapsed: oneshot::AsyncReceiver<()>,
}

impl Future for SlintSleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project().elapsed.poll(cx) {
            Poll::Ready(_) => Poll::Ready(()),
            Poll::Pending => Poll::Pending,
        }
    }
}

#[derive(Debug)]
#[pin_project]
pub struct SlintTimeout<F> {
    #[pin]
    timeout: oneshot::AsyncReceiver<()>,
    #[pin]
    future: F,
}

impl<F> Future for SlintTimeout<F>
where
    F: Future,
{
    type Output = Result<F::Output, TimeoutError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let proj = self.project();
        match proj.future.poll(cx) {
            Poll::Ready(value) => Poll::Ready(Ok(value)),
            Poll::Pending => match proj.timeout.poll(cx) {
                Poll::Ready(_) => Poll::Ready(Err(TimeoutError)),
                Poll::Pending => Poll::Pending,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::sync::{Arc, OnceLock};

    use googletest::{assert_that, matchers::*};

    static EVENT_LOOP_INITIALIZED: OnceLock<std::thread::JoinHandle<()>> = OnceLock::new();

    fn init_event_loop() {
        EVENT_LOOP_INITIALIZED.get_or_init(|| std::thread::spawn(|| slint::run_event_loop_until_quit().unwrap()));
        while slint::invoke_from_event_loop(|| {}).is_err() {
            std::thread::sleep(Duration::from_millis(16));
        }
    }

    #[test]
    fn spawn() {
        init_event_loop();

        let runtime = SlintRuntime;
        let (tx, rx) = oneshot::channel();
        runtime.spawn(async {
            let _ = tx.send(());
        });
        assert_that!(rx.recv_timeout(Duration::from_secs(5)), ok(anything()));
    }

    #[test]
    fn spawn_child() {
        init_event_loop();

        let runtime = Arc::new(SlintRuntime);
        let runtime_ = runtime.clone();
        let (tx, rx) = oneshot::channel();
        runtime.spawn(async move {
            let result = runtime_.spawn(async {}).await;
            let _ = tx.send(result);
        });
        assert_that!(rx.recv_timeout(Duration::from_secs(5)), ok(ok(eq(&()))));
    }

    #[test]
    fn sleep() {
        init_event_loop();

        let runtime = Arc::new(SlintRuntime);
        let (tx, rx) = oneshot::channel();
        runtime.clone().spawn(async move {
            let start = Instant::now();
            let _ = runtime.sleep(Duration::from_millis(32)).await;
            let end = Instant::now();
            let _ = tx.send(end - start);
        });
        assert_that!(rx.recv_timeout(Duration::from_secs(5)), ok(gt(Duration::from_millis(2))));
    }

    #[test]
    fn sleep_until() {
        init_event_loop();

        let runtime = Arc::new(SlintRuntime);
        let (tx, rx) = oneshot::channel();
        runtime.clone().spawn(async move {
            let start = Instant::now();
            let _ = runtime.sleep_until(start + Duration::from_millis(32)).await;
            let end = Instant::now();
            let _ = tx.send(end - start);
        });
        assert_that!(rx.recv_timeout(Duration::from_secs(5)), ok(gt(Duration::from_millis(2))));
    }

    #[test]
    fn timeout_completed() {
        init_event_loop();

        let runtime = Arc::new(SlintRuntime);
        let (tx, rx) = oneshot::channel();
        runtime.clone().spawn(async move {
            let result = runtime.timeout(Duration::from_millis(16), async {}).await;
            let _ = tx.send(result);
        });
        assert_that!(rx.recv_timeout(Duration::from_secs(5)), ok(ok(eq(&()))));
    }

    #[test]
    fn timeout_failed() {
        init_event_loop();

        let runtime = Arc::new(SlintRuntime);
        let (tx, rx) = oneshot::channel();
        runtime.clone().spawn(async move {
            let result = runtime
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
        init_event_loop();

        let runtime = Arc::new(SlintRuntime);
        let (tx, rx) = oneshot::channel();
        runtime.clone().spawn(async move {
            let result = runtime.timeout_at(Instant::now() + Duration::from_millis(16), async {}).await;
            let _ = tx.send(result);
        });
        assert_that!(rx.recv_timeout(Duration::from_secs(5)), ok(ok(eq(&()))));
    }

    #[test]
    fn timeout_at_failed() {
        init_event_loop();

        let runtime = Arc::new(SlintRuntime);
        let (tx, rx) = oneshot::channel();
        runtime.clone().spawn(async move {
            let result = runtime
                .timeout_at(Instant::now() + Duration::from_millis(16), async {
                    std::future::pending::<()>().await;
                })
                .await;
            let _ = tx.send(result);
        });
        assert_that!(rx.recv_timeout(Duration::from_secs(5)), ok(err(eq(&TimeoutError))));
    }
}
