use std::collections::VecDeque as Queue;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

use crate::async_finalize::async_finalize;
use crate::async_finalize::{sync_finalize, AsyncFinalize};
use crate::rpc::error::Error;
use crate::rpc::method::{MethodCall, MethodResult};
use crate::rpc::protocol::MethodCaller;
use crate::rpc::PackagedMethod;
use crate::specification::methods;

pub struct SPSession {
    method_caller: MethodCaller,
    response_queue: Mutex<Queue<oneshot::Sender<Result<PackagedMethod, Error>>>>,
}

impl SPSession {
    pub fn new(method_caller: MethodCaller) -> Self {
        Self { method_caller, response_queue: Queue::new().into() }
    }

    pub async fn call(&self, request: MethodCall) -> Result<MethodResult, Error> {
        if let PackagedMethod::Result(result) = self.call_generic(PackagedMethod::Call(request)).await? {
            Ok(result)
        } else {
            Err(Error::MethodResultExpected)
        }
    }

    pub async fn end(&self) -> Result<(), Error> {
        if let PackagedMethod::EndOfSession = self.call_generic(PackagedMethod::EndOfSession).await? {
            Ok(())
        } else {
            Err(Error::EOSExpected)
        }
    }
}

impl SPSession {
    async fn call_generic(&self, request: PackagedMethod) -> Result<PackagedMethod, Error> {
        let rx = self.send_one(request).await?;
        self.recv_one().await;
        match rx.await {
            Ok(result) => result,
            Err(_) => Err(Error::Closed),
        }
    }

    async fn send_one(
        &self,
        request: PackagedMethod,
    ) -> Result<oneshot::Receiver<Result<PackagedMethod, Error>>, Error> {
        let mut response_queue = self.response_queue.lock().await;
        match self.method_caller.send(request).await {
            Ok(_) => {
                let (tx, rx) = oneshot::channel();
                // Must be under the same lock as the call to send!
                // This ensures the correct pairing between the method call and its result.
                response_queue.push_back(tx);
                Ok(rx)
            }
            Err(err) => Err(err),
        }
    }

    async fn recv_one(&self) {
        let response = self.method_caller.recv().await;
        let mut response_queue = self.response_queue.lock().await;
        if let Some(tx) = response_queue.pop_front() {
            let _ = tx.send(decode_response(response));
        };
    }
}

fn decode_response(response: Result<PackagedMethod, Error>) -> Result<PackagedMethod, Error> {
    match response {
        Ok(PackagedMethod::Call(call)) => {
            if call.method_id == methods::CLOSE_SESSION {
                // CloseSession
                Err(Error::AbortedByRemote)
            } else {
                Err(Error::MethodResultExpected)
            }
        }
        Ok(packaged_method) => Ok(packaged_method),
        Err(err) => Err(err),
    }
}

impl AsyncFinalize for SPSession {
    async fn finalize(&mut self) {
        async_finalize(&mut self.method_caller).await;
    }
}

impl Drop for SPSession {
    fn drop(&mut self) {
        sync_finalize(self);
    }
}
