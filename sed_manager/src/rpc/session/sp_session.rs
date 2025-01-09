use std::collections::VecDeque as Queue;
use std::sync::Arc;
use tokio::sync::oneshot;
use tokio::sync::Mutex;

use crate::messaging::uid::UID;
use crate::rpc::error::Error;
use crate::rpc::method::{MethodCall, MethodResult};
use crate::rpc::protocol::MethodCaller;
use crate::rpc::protocol::PackagedMethod;

pub struct SPSession {
    method_layer: Arc<MethodCaller>,
    response_queue: Arc<Mutex<Queue<oneshot::Sender<Result<MethodResult, Error>>>>>,
}

impl SPSession {
    pub fn new(method_layer: MethodCaller) -> Self {
        Self { method_layer: Arc::new(method_layer), response_queue: Arc::new(Queue::new().into()) }
    }

    pub async fn call(&self, request: MethodCall) -> Result<MethodResult, Error> {
        let rx = {
            // Sending the request and enqueueing the response sender under the same lock
            // ensures the pairing of the send and recv calls.
            let mut response_queue = self.response_queue.lock().await;
            let (tx, rx) = oneshot::channel();
            response_queue.push_back(tx);
            self.method_layer.send(PackagedMethod::Call(request)).await?;
            rx
        };
        let interface_layer = self.method_layer.clone();
        let response_queue = self.response_queue.clone();
        let task = tokio::spawn(async move {
            let response = interface_layer.recv().await;
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
        self.method_layer.close().await
    }
}

fn decode_response(response: Result<PackagedMethod, Error>) -> Result<MethodResult, Error> {
    match response {
        Ok(PackagedMethod::Call(call)) => {
            if call.method_id == UID::from(0xFF06_u64) {
                // CloseSession
                Err(Error::AbortedByRemote)
            } else {
                Err(Error::MethodResultExpected)
            }
        }
        Ok(PackagedMethod::Result(result)) => Ok(result),
        Ok(PackagedMethod::EndOfSession) => Err(Error::Closed),
        Err(err) => Err(err),
    }
}
