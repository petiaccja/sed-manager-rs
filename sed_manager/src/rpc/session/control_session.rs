use tokio::sync::Mutex;

use crate::rpc::error::Error;
use crate::rpc::method::MethodCall;
use crate::rpc::protocol::{MethodCaller, PackagedMethod};

pub struct ControlSession {
    method_caller: Mutex<MethodCaller>,
}

impl ControlSession {
    pub fn new(method_layer: MethodCaller) -> Self {
        Self { method_caller: method_layer.into() }
    }

    pub async fn call(&self, method: MethodCall) -> Result<MethodCall, Error> {
        // The mutex is needed because:
        //  - The device drops management layer calls if they are invalid.
        //  - Therefore the only way to detect failure is timeout.
        //  - With multiple requests in flight you don't know which one timed out.
        // Note: matching is still possible by looking into the messages:
        //  - Properties calls can be matched.
        //  - StartSession and SyncSession calls can be matched by HSN.
        let method_caller = self.method_caller.lock().await;
        method_caller.send(PackagedMethod::Call(method)).await?;
        let response = method_caller.recv().await?;
        match response {
            PackagedMethod::Call(response) => Ok(response),
            PackagedMethod::Result(_) => Err(Error::MethodCallExpected),
            PackagedMethod::EndOfSession => Err(Error::Closed),
        }
    }

    pub async fn close(&self) {
        self.method_caller.lock().await.close().await
    }
}
