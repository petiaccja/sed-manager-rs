use core::future::Future;
use core::{any::Any, pin::Pin, time::Duration};

use crate::rpc::Error;

pub trait Runtime: Send + Sync {
    fn spawn<F>(&self, future: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + Any;

    fn timeout<'fut, F>(
        &self,
        duration: Duration,
        future: F,
    ) -> Pin<Box<dyn Future<Output = Result<F::Output, Error>> + Send + 'fut>>
    where
        F: Future + Send + 'fut,
        F::Output: Send + Any;
}

pub trait DynamicRuntime: Send + Sync {
    fn spawn_dynamic(&self, future: Pin<Box<dyn Future<Output = Box<dyn Any + Send>> + Send>>);

    fn timeout_dynamic<'fut>(
        &self,
        duration: Duration,
        future: Pin<Box<dyn Future<Output = Box<dyn Any + Send>> + Send + 'fut>>,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn Any + Send>, Error>> + Send + 'fut>>;
}

impl<ConcreteRuntime> DynamicRuntime for ConcreteRuntime
where
    ConcreteRuntime: Runtime,
{
    fn spawn_dynamic(&self, future: Pin<Box<dyn Future<Output = Box<dyn Any + Send>> + Send>>) {
        self.spawn(future);
    }

    fn timeout_dynamic<'fut>(
        &self,
        duration: Duration,
        future: Pin<Box<dyn Future<Output = Box<dyn Any + Send>> + Send + 'fut>>,
    ) -> Pin<Box<dyn Future<Output = Result<Box<dyn Any + Send>, Error>> + Send + 'fut>> {
        self.timeout(duration, future)
    }
}

impl Runtime for dyn DynamicRuntime {
    fn spawn<F>(&self, future: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + Any,
    {
        let future = Box::pin(async move { Box::new(future.await) as Box<dyn Any + Send> });
        DynamicRuntime::spawn_dynamic(self, future);
    }

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
        let to_future = DynamicRuntime::timeout_dynamic(self, duration, future);
        Box::pin(async move {
            let result = to_future.await;
            result.map(|result| *result.downcast::<F::Output>().unwrap())
        })
    }
}

pub struct TokioRuntime {
    runtime: Option<tokio::runtime::Runtime>,
}

impl TokioRuntime {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .enable_all()
            .worker_threads(1)
            .thread_name("protocol-runtime")
            .build()
            .expect("building tokio runtime failed");
        Self { runtime: Some(runtime) }
    }
}

impl Runtime for TokioRuntime {
    fn spawn<F>(&self, future: F)
    where
        F: Future + Send + 'static,
        F::Output: Send + Any,
    {
        let _ = self.runtime.as_ref().unwrap().spawn(future);
    }

    fn timeout<'fut, F>(
        &self,
        duration: Duration,
        future: F,
    ) -> Pin<Box<dyn Future<Output = Result<F::Output, Error>> + Send + 'fut>>
    where
        F: Future + Send + 'fut,
        F::Output: Send + Any,
    {
        let timeout_fut = tokio::time::timeout(duration, future);
        Box::pin(async move { timeout_fut.await.map_err(|_| Error::TimedOut) })
    }
}

impl Drop for TokioRuntime {
    fn drop(&mut self) {
        // Need to wait for spawned protocol stacks to exit or else they will
        // likely leave the TPer in an inconsistent state, dropping packets,
        // and requiring a stack reset.
        self.runtime.take().unwrap().shutdown_timeout(Duration::from_secs(30));
    }
}
