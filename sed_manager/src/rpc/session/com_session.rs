use std::sync::mpsc;
use tokio::sync::{oneshot, Mutex};

use crate::messaging::com_id::{HandleComIdRequest, HandleComIdResponse};
use crate::rpc::error::Error;
use crate::rpc::protocol::{Message, Tracked};

pub struct ComSession {
    sender: mpsc::Sender<Message>,
    mutex: Mutex<()>,
}

impl ComSession {
    pub fn new(sender: mpsc::Sender<Message>) -> Self {
        Self { sender, mutex: ().into() }
    }

    pub async fn handle_request(&self, request: HandleComIdRequest) -> Result<HandleComIdResponse, Error> {
        let (tx, rx) = oneshot::channel();
        let _guard = self.mutex.lock().await;
        let _ = self.sender.send(Message::HandleComId { content: Tracked { item: request, promises: vec![tx] } });
        match rx.await {
            Ok(result) => result,
            Err(_) => Err(Error::Closed),
        }
    }
}
