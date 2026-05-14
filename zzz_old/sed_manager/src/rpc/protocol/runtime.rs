//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::future::Future;
use core::{any::Any, pin::Pin, time::Duration};

use crate::rpc::Error;

pub trait Runtime: Send + Sync + Any {
    fn spawn<F>(&self, future: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + Any;

    fn timer(&self) -> impl Timer + 'static;
}

pub trait Timer: Send + Sync {
    fn timeout<'fut, F>(
        &self,
        duration: Duration,
        future: F,
    ) -> Pin<Box<dyn Future<Output = Result<F::Output, Error>> + Send + 'fut>>
    where
        F: Future + Send + 'fut,
        F::Output: Send + Any;
}

pub trait DynamicTimer: Send + Sync + 'static {
    fn timeout_dynamic<'fut>(
        &self,
        duration: Duration,
        future: Pin<Box<dyn Future<Output = Box<dyn Any + Send>> + Send + 'fut>>,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn Any + Send>, Error>> + Send + 'fut>>;
}

impl<ConcreteTimer> DynamicTimer for ConcreteTimer
where
    ConcreteTimer: Timer + 'static,
{
    fn timeout_dynamic<'fut>(
        &self,
        duration: Duration,
        future: Pin<Box<dyn Future<Output = Box<dyn Any + Send>> + Send + 'fut>>,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn Any + Send>, Error>> + Send + 'fut>> {
        self.timeout(duration, future)
    }
}

impl Timer for dyn DynamicTimer {
    fn timeout<'fut, F>(
        &self,
        duration: Duration,
        future: F,
    ) -> Pin<Box<dyn Future<Output = Result<F::Output, Error>> + Send + 'fut>>
    where
        F: Future + Send + 'fut,
        F::Output: Send + Any,
    {
        let future = Box::pin(async move { Box::new(future.await) as Box<dyn Any + Send> });
        let to_future = self.timeout_dynamic(duration, future);
        Box::pin(async move {
            let result = to_future.await;
            result.map(|result| *result.downcast::<F::Output>().unwrap())
        })
    }
}

pub struct TokioRuntime {
    runtime: Option<tokio::runtime::Runtime>,
    task_tracker: Option<tokio_util::task::TaskTracker>,
}

pub struct TokioTimer {}

impl TokioRuntime {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .enable_all()
            .worker_threads(1)
            .thread_name("protocol-runtime")
            .build()
            .expect("building tokio runtime failed");
        let task_tracker = tokio_util::task::TaskTracker::new();
        Self { runtime: Some(runtime), task_tracker: Some(task_tracker) }
    }
}

impl Runtime for TokioRuntime {
    fn spawn<F>(&self, future: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + Any,
    {
        let runtime = self.runtime.as_ref().unwrap();
        let task_tracker = self.task_tracker.as_ref().unwrap();
        let _ = task_tracker.spawn_on(future, runtime.handle());
    }

    fn timer(&self) -> impl Timer + 'static {
        TokioTimer {}
    }
}

impl Timer for TokioTimer {
    fn timeout<'fut, F>(
        &self,
        duration: Duration,
        future: F,
    ) -> Pin<Box<dyn Future<Output = Result<F::Output, Error>> + Send + 'fut>>
    where
        F: Future + Send + 'fut,
        F::Output: Send + Any,
    {
        Box::pin(async move { tokio::time::timeout(duration, future).await.map_err(|_| Error::TimedOut) })
    }
}

impl Drop for TokioRuntime {
    fn drop(&mut self) {
        // The protocol stacks spawned on the runtime need to finish or else they can
        // leave the device in an inconsistent state.
        // This is ensured by waiting for all spawned tasks to finish. Spawned tasks are tracked
        // by the `task_tracker`.

        tracing::event!(tracing::Level::DEBUG, "Protocol TokioRuntime: drop");

        // This is incredibly dumb... Tokio won't let you `block_on` within a runtime, even
        // if the two runtimes are totally unrelated. Furthermore, you also cannot drop a
        // runtime from another runtime, even if the two runtimes are totally unrelated.
        // This conveniently break all `[tokio::test]` tests, so we need to spawn
        // a separate thread, drop the runtime on that, and block on this runtime
        // all the same with thread.join().
        let runtime = self.runtime.take().unwrap();
        let task_tracker = self.task_tracker.take().unwrap();
        let handle = std::thread::spawn(move || {
            task_tracker.close();
            runtime.block_on(task_tracker.wait());
        });
        let _ = handle.join();
    }
}
