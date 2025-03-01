use core::future::Future;
use std::time::Duration;

pub struct ProtocolRuntime {
    runtime: Option<tokio::runtime::Runtime>,
}

impl ProtocolRuntime {
    pub fn new() -> Self {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_time()
            .thread_name("rpc-protocol")
            .worker_threads(1)
            .build()
            .expect("failed to initialize async runtime");
        Self { runtime: Some(runtime) }
    }

    pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output>
    where
        F: Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.runtime.as_ref().expect("runtime should always be Some").spawn(future)
    }
}

impl Drop for ProtocolRuntime {
    fn drop(&mut self) {
        self.runtime
            .take()
            .expect("runtime should be dropped only once")
            .shutdown_timeout(Duration::from_secs(30));
    }
}

pub static RUNTIME: std::sync::LazyLock<ProtocolRuntime> = std::sync::LazyLock::new(|| ProtocolRuntime::new());
