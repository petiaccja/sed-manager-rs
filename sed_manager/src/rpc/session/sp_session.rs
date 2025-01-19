use std::collections::VecDeque as Queue;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

use crate::rpc::error::Error;
use crate::rpc::method::{MethodCall, MethodResult};
use crate::rpc::protocol::MethodCaller;
use crate::rpc::protocol::PackagedMethod;
use crate::specification::methods;

pub struct SPSession {
    method_caller: Arc<MethodCaller>,
    response_queue: Arc<Mutex<Queue<oneshot::Sender<Result<PackagedMethod, Error>>>>>,
}

impl SPSession {
    pub fn new(method_layer: MethodCaller) -> Self {
        Self { method_caller: Arc::new(method_layer), response_queue: Arc::new(Queue::new().into()) }
    }

    pub async fn call_method(&self, request: MethodCall) -> Result<MethodResult, Error> {
        if let PackagedMethod::Result(result) = self.call(PackagedMethod::Call(request)).await? {
            Ok(result)
        } else {
            Err(Error::MethodResultExpected)
        }
    }

    pub async fn call_eos(&self) -> Result<(), Error> {
        if let PackagedMethod::EndOfSession = self.call(PackagedMethod::EndOfSession).await? {
            Ok(())
        } else {
            Err(Error::EOSExpected)
        }
    }

    pub async fn call(&self, request: PackagedMethod) -> Result<PackagedMethod, Error> {
        let rx = {
            // Sending the request and enqueueing the response sender under the same lock
            // ensures the pairing of the send and recv calls.
            let mut response_queue = self.response_queue.lock().await;
            let (tx, rx) = oneshot::channel();
            response_queue.push_back(tx);
            self.method_caller.send(request).await?;
            rx
        };
        let method_caller = self.method_caller.clone();
        let response_queue = self.response_queue.clone();
        let task = tokio::spawn(async move {
            let response = method_caller.recv().await;
            let mut response_queue = response_queue.lock().await;
            if let Some(tx) = response_queue.pop_front() {
                let _ = tx.send(decode_response(response));
            };
        });
        let _ = task.await;
        match rx.await {
            Ok(result) => result,
            Err(_) => Err(Error::Closed),
        }
    }

    pub async fn close(&self) {
        self.method_caller.close().await
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
