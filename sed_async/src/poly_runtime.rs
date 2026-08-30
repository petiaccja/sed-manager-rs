use std::{
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
    time::{Duration, Instant},
};

use pin_project::pin_project;

#[cfg(feature = "slint")]
use crate::slint_runtime::SlintRuntime;
#[cfg(feature = "tokio")]
use crate::tokio_runtime::TokioRuntime;
use crate::{
    Runtime,
    runtime::{JoinError, ShutdownError, TimeoutError},
};

#[derive(Debug)]
pub enum PolyRuntime {
    #[cfg(feature = "tokio")]
    Tokio(TokioRuntime),
    #[cfg(feature = "slint")]
    Slint(SlintRuntime),
}

impl Runtime for PolyRuntime {
    type JoinHandle<T> = PolyJoinHandle<T>;
    type Sleep = PolySleep;
    type SleepUntil = PolySleepUntil;
    type Timeout<F: Future> = PolyTimeout<F>;
    type TimeoutAt<F: Future> = PolyTimeoutAt<F>;

    fn block_on<F>(&self, f: F) -> F::Output
    where
        F: Future + Send + 'static,
        F::Output: Send,
    {
        #[cfg(feature = "not_empty")]
        match self {
            #[cfg(feature = "tokio")]
            PolyRuntime::Tokio(inner) => inner.block_on(f),
            #[cfg(feature = "slint")]
            PolyRuntime::Slint(inner) => inner.block_on(f),
        }
        #[cfg(not(feature = "not_empty"))]
        {
            std::hint::black_box(f);
            unreachable!()
        }
    }

    fn spawn<F>(&self, f: F) -> Self::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        #[cfg(feature = "not_empty")]
        match self {
            #[cfg(feature = "tokio")]
            PolyRuntime::Tokio(inner) => Self::JoinHandle::Tokio(inner.spawn(f)),
            #[cfg(feature = "slint")]
            PolyRuntime::Slint(inner) => Self::JoinHandle::Slint(inner.spawn(f)),
        }
        #[cfg(not(feature = "not_empty"))]
        {
            std::hint::black_box(f);
            unreachable!()
        }
    }

    fn shutdown(self, duration: Duration) -> Result<(), ShutdownError> {
        #[cfg(feature = "not_empty")]
        match self {
            #[cfg(feature = "tokio")]
            PolyRuntime::Tokio(inner) => inner.shutdown(duration),
            #[cfg(feature = "slint")]
            PolyRuntime::Slint(inner) => inner.shutdown(duration),
        }
        #[cfg(not(feature = "not_empty"))]
        {
            std::hint::black_box(duration);
            unreachable!()
        }
    }

    fn yield_now(&self) -> impl Future<Output = ()> {
        async {
            #[cfg(feature = "not_empty")]
            match self {
                #[cfg(feature = "tokio")]
                PolyRuntime::Tokio(inner) => inner.yield_now().await,
                #[cfg(feature = "slint")]
                PolyRuntime::Slint(inner) => inner.yield_now().await,
            }
            #[cfg(not(feature = "not_empty"))]
            {
                unreachable!()
            }
        }
    }

    fn sleep(&self, duration: Duration) -> Self::Sleep {
        #[cfg(feature = "not_empty")]
        match self {
            #[cfg(feature = "tokio")]
            PolyRuntime::Tokio(inner) => Self::Sleep::Tokio(inner.sleep(duration)),
            #[cfg(feature = "slint")]
            PolyRuntime::Slint(inner) => Self::Sleep::Slint(inner.sleep(duration)),
        }
        #[cfg(not(feature = "not_empty"))]
        {
            std::hint::black_box(duration);
            unreachable!()
        }
    }

    fn sleep_until(&self, time: Instant) -> Self::SleepUntil {
        #[cfg(feature = "not_empty")]
        match self {
            #[cfg(feature = "tokio")]
            PolyRuntime::Tokio(inner) => Self::SleepUntil::Tokio(inner.sleep_until(time)),
            #[cfg(feature = "slint")]
            PolyRuntime::Slint(inner) => Self::SleepUntil::Slint(inner.sleep_until(time)),
        }
        #[cfg(not(feature = "not_empty"))]
        {
            std::hint::black_box(time);
            unreachable!()
        }
    }

    fn timeout<F>(&self, duration: Duration, future: F) -> Self::Timeout<F>
    where
        F: Future,
    {
        #[cfg(feature = "not_empty")]
        match self {
            #[cfg(feature = "tokio")]
            PolyRuntime::Tokio(inner) => Self::Timeout::Tokio(inner.timeout(duration, future)),
            #[cfg(feature = "slint")]
            PolyRuntime::Slint(inner) => Self::Timeout::Slint(inner.timeout(duration, future)),
        }
        #[cfg(not(feature = "not_empty"))]
        {
            std::hint::black_box((duration, future));
            unreachable!()
        }
    }

    fn timeout_at<F>(&self, time: Instant, future: F) -> Self::TimeoutAt<F>
    where
        F: Future,
    {
        #[cfg(feature = "not_empty")]
        match self {
            #[cfg(feature = "tokio")]
            PolyRuntime::Tokio(inner) => Self::TimeoutAt::Tokio(inner.timeout_at(time, future)),
            #[cfg(feature = "slint")]
            PolyRuntime::Slint(inner) => Self::TimeoutAt::Slint(inner.timeout_at(time, future)),
        }
        #[cfg(not(feature = "not_empty"))]
        {
            std::hint::black_box((time, future));
            unreachable!()
        }
    }
}

#[derive(Debug)]
#[pin_project(project = PolyJoinHandleProj)]
pub enum PolyJoinHandle<T> {
    #[cfg(feature = "slint")]
    Slint(#[pin] <SlintRuntime as Runtime>::JoinHandle<T>),
    #[cfg(feature = "tokio")]
    Tokio(#[pin] <TokioRuntime as Runtime>::JoinHandle<T>),
    Noop(PhantomData<T>),
}

impl<T> Future for PolyJoinHandle<T> {
    type Output = Result<T, JoinError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            #[cfg(feature = "slint")]
            PolyJoinHandleProj::Slint(inner) => inner.poll(cx),
            #[cfg(feature = "tokio")]
            PolyJoinHandleProj::Tokio(inner) => inner.poll(cx),
            PolyJoinHandleProj::Noop(_) => {
                std::hint::black_box(cx);
                unreachable!()
            }
        }
    }
}

#[pin_project(project = PolySleepProj)]
pub enum PolySleep {
    #[cfg(feature = "slint")]
    Slint(#[pin] <SlintRuntime as Runtime>::Sleep),
    #[cfg(feature = "tokio")]
    Tokio(#[pin] <TokioRuntime as Runtime>::Sleep),
    Noop(#[pin] PhantomData<()>),
}

impl Future for PolySleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            #[cfg(feature = "slint")]
            PolySleepProj::Slint(inner) => inner.poll(cx),
            #[cfg(feature = "tokio")]
            PolySleepProj::Tokio(inner) => inner.poll(cx),
            PolySleepProj::Noop(_) => {
                std::hint::black_box(cx);
                unreachable!()
            }
        }
    }
}

#[pin_project(project = PolySleepUntilProj)]
pub enum PolySleepUntil {
    #[cfg(feature = "slint")]
    Slint(#[pin] <SlintRuntime as Runtime>::SleepUntil),
    #[cfg(feature = "tokio")]
    Tokio(#[pin] <TokioRuntime as Runtime>::SleepUntil),
    Noop(#[pin] PhantomData<()>),
}

impl Future for PolySleepUntil {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            #[cfg(feature = "slint")]
            PolySleepUntilProj::Slint(inner) => inner.poll(cx),
            #[cfg(feature = "tokio")]
            PolySleepUntilProj::Tokio(inner) => inner.poll(cx),
            PolySleepUntilProj::Noop(_) => {
                std::hint::black_box(cx);
                unreachable!()
            }
        }
    }
}

#[pin_project(project = PolyTimeoutProj)]
pub enum PolyTimeout<F: Future> {
    #[cfg(feature = "slint")]
    Slint(#[pin] <SlintRuntime as Runtime>::Timeout<F>),
    #[cfg(feature = "tokio")]
    Tokio(#[pin] <TokioRuntime as Runtime>::Timeout<F>),
    Noop(PhantomData<F>),
}

impl<F: Future> Future for PolyTimeout<F> {
    type Output = Result<F::Output, TimeoutError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            #[cfg(feature = "slint")]
            PolyTimeoutProj::Slint(inner) => inner.poll(cx),
            #[cfg(feature = "tokio")]
            PolyTimeoutProj::Tokio(inner) => inner.poll(cx),
            PolyTimeoutProj::Noop(_) => {
                std::hint::black_box(cx);
                unreachable!()
            }
        }
    }
}

#[pin_project(project = PolyTimeoutAtProj)]
pub enum PolyTimeoutAt<F: Future> {
    #[cfg(feature = "slint")]
    Slint(#[pin] <SlintRuntime as Runtime>::TimeoutAt<F>),
    #[cfg(feature = "tokio")]
    Tokio(#[pin] <TokioRuntime as Runtime>::TimeoutAt<F>),
    Noop(PhantomData<F>),
}

impl<F: Future> Future for PolyTimeoutAt<F> {
    type Output = Result<F::Output, TimeoutError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.project() {
            #[cfg(feature = "slint")]
            PolyTimeoutAtProj::Slint(inner) => inner.poll(cx),
            #[cfg(feature = "tokio")]
            PolyTimeoutAtProj::Tokio(inner) => inner.poll(cx),
            PolyTimeoutAtProj::Noop(_) => {
                std::hint::black_box(cx);
                unreachable!()
            }
        }
    }
}
