use tokio::sync::Mutex;

use crate::async_finalize::{async_finalize, sync_finalize, AsyncFinalize};
use crate::rpc::error::Error;
use crate::rpc::method::MethodCall;
use crate::rpc::protocol::MethodCaller;
use crate::rpc::PackagedMethod;
use crate::specification::method;

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
        loop {
            let response = method_caller.recv().await?;
            break match response {
                PackagedMethod::Call(response) => {
                    // Ignore CloseSession calls that the TPer may send when it aborts an
                    // SP session. The only way to make use of CloseSession packets is to
                    // decode them at the method call level right at the SessionRouter level,
                    // which is ridiculously inconvenient as the higher layers *still* need
                    // packets instead of methods. Sometimes I really wonder if *anybody*
                    // tried to implement this specification before they published it.
                    // Doesn't seem like so...
                    if response.method_id != method::CLOSE_SESSION {
                        Ok(response)
                    } else {
                        continue;
                    }
                }
                PackagedMethod::Result(_) => Err(Error::MethodCallExpected),
                PackagedMethod::EndOfSession => Err(Error::Closed),
            };
        }
    }
}

impl AsyncFinalize for ControlSession {
    async fn finalize(&mut self) {
        async_finalize(self.method_caller.get_mut()).await;
    }
}

impl Drop for ControlSession {
    fn drop(&mut self) {
        sync_finalize(self);
    }
}
