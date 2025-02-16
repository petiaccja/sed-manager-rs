use std::sync::mpsc;
use tokio::sync::{oneshot, Mutex};

use crate::rpc::error::{Error, ErrorEvent, ErrorEventExt};
use crate::rpc::method::MethodCall;
use crate::rpc::protocol::{Message, SessionIdentifier, Tracked};
use crate::rpc::{PackagedMethod, Properties};

const CONTROL_SESSION_ID: SessionIdentifier = SessionIdentifier { hsn: 0, tsn: 0 };

pub struct ControlSession {
    sender: mpsc::Sender<Message>,
    mutex: Mutex<()>,
}

impl ControlSession {
    pub fn new(sender: mpsc::Sender<Message>) -> Self {
        let _ = sender.send(Message::StartSession { session: CONTROL_SESSION_ID, properties: Properties::ASSUMED });
        Self { sender, mutex: ().into() }
    }

    pub async fn call(&self, method: MethodCall) -> Result<MethodCall, Error> {
        let (tx, rx) = oneshot::channel();
        let content = Tracked { item: PackagedMethod::Call(method), promises: vec![tx] };
        let _guard = self.mutex.lock().await;
        let _ = self.sender.send(Message::Method { session: CONTROL_SESSION_ID, content });
        match rx.await {
            Ok(Ok(PackagedMethod::Call(result))) => Ok(result),
            Ok(Ok(_)) => Err(ErrorEvent::MethodCallExpected.while_receiving()),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(ErrorEvent::Closed.while_receiving()),
        }
    }
}

impl Drop for ControlSession {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::EndSession { session: CONTROL_SESSION_ID });
    }
}
