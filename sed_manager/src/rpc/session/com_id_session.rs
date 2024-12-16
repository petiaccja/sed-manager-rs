use std::collections::VecDeque as Queue;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

use crate::messaging::com_id::{HandleComIdRequest, HandleComIdResponse};
use crate::rpc::error::Error;
use crate::rpc::protocol::InterfaceLayer;

pub struct ComIdSession {
    interface_layer: Arc<dyn InterfaceLayer>,
    response_queue: Arc<Mutex<Queue<oneshot::Sender<Result<HandleComIdResponse, Error>>>>>,
}

impl ComIdSession {
    pub fn new(interface_layer: Arc<dyn InterfaceLayer>) -> Self {
        Self { interface_layer: interface_layer.into(), response_queue: Arc::new(Queue::new().into()) }
    }

    pub async fn handle_request(&self, request: HandleComIdRequest) -> Result<HandleComIdResponse, Error> {
        let rx = {
            // Sending the request and enqueueing the response sender under the same lock
            // ensures the pairing of the send and recv calls.
            let mut response_queue = self.response_queue.lock().await;
            let (tx, rx) = oneshot::channel();
            response_queue.push_back(tx);
            self.interface_layer.send_handle_com_id(request).await?;
            rx
        };
        let interface_layer = self.interface_layer.clone();
        let response_queue = self.response_queue.clone();
        let task = tokio::spawn(async move {
            let result = interface_layer.recv_handle_com_id().await;
            let mut response_queue = response_queue.lock().await;
            if let Some(tx) = response_queue.pop_front() {
                let _ = tx.send(result);
            };
        });
        let _ = task.await;
        match rx.await {
            Ok(result) => result,
            Err(_) => Err(Error::Closed),
        }
    }

    pub async fn close(&self) {
        ()
    }
}
