use tokio::sync::Mutex;

use crate::rpc::error::Error;
use crate::rpc::method::MethodCall;
use crate::rpc::protocol::{MethodLayer, PackagedMethod};

pub struct ManagementSession {
    method_layer: Mutex<MethodLayer>,
}

impl ManagementSession {
    pub fn new(method_layer: MethodLayer) -> Self {
        Self { method_layer: method_layer.into() }
    }

    pub async fn call(&self, method: MethodCall) -> Result<MethodCall, Error> {
        // The mutex is needed because:
        //  - The device drops management layer calls if they are invalid.
        //  - Therefore the only way to detect failure is timeout.
        //  - With multiple requests in flight you don't know which one timed out.
        // Note: matching is still possible by looking into the messages:
        //  - Properties calls can be matched.
        //  - StartSession and SyncSession calls can be matched by HSN.
        let method_layer = self.method_layer.lock().await;
        method_layer.send(PackagedMethod::Call(method)).await?;
        let response = method_layer.recv().await?;
        match response {
            PackagedMethod::Call(response) => Ok(response),
            PackagedMethod::Result(_) => Err(Error::MethodCallExpected),
            PackagedMethod::EndOfSession => Err(Error::Closed),
        }
    }

    pub async fn close(&self) {
        self.method_layer.lock().await.close().await
    }
}
