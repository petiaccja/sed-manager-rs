use std::sync::mpsc;
use tokio::sync::oneshot;

use crate::rpc::error::{Error, ErrorEvent, ErrorEventExt};
use crate::rpc::method::MethodCall;
use crate::rpc::protocol::{Message, SessionIdentifier, Tracked};
use crate::rpc::{MethodResult, PackagedMethod, Properties};

pub struct SPSession {
    session: SessionIdentifier,
    sender: mpsc::Sender<Message>,
}

impl SPSession {
    pub fn new(hsn: u32, tsn: u32, properties: Properties, sender: mpsc::Sender<Message>) -> Self {
        let session = SessionIdentifier { hsn, tsn };
        let _ = sender.send(Message::StartSession { session, properties });
        Self { session, sender }
    }

    pub async fn call(&self, method: MethodCall) -> Result<MethodResult, Error> {
        let (tx, rx) = oneshot::channel();
        let content = Tracked { item: PackagedMethod::Call(method), promises: vec![tx] };
        let _ = self.sender.send(Message::Method { session: self.session, content });
        match rx.await {
            Ok(Ok(PackagedMethod::Result(result))) => Ok(result),
            Ok(Ok(_)) => Err(ErrorEvent::MethodResultExpected.while_receiving()),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(ErrorEvent::Closed.while_receiving()),
        }
    }

    pub async fn end(&self) -> Result<(), Error> {
        let (tx, rx) = oneshot::channel();
        let content = Tracked { item: PackagedMethod::EndOfSession, promises: vec![tx] };
        let _ = self.sender.send(Message::Method { session: self.session, content });
        match rx.await {
            Ok(Ok(PackagedMethod::EndOfSession)) => Ok(()),
            Ok(Ok(_)) => Err(ErrorEvent::EOSExpected.while_receiving()),
            Ok(Err(err)) => Err(err),
            Err(_) => Err(ErrorEvent::Closed.while_receiving()),
        }
    }
}

impl Drop for SPSession {
    fn drop(&mut self) {
        let (tx, _rx) = oneshot::channel();
        let content = Tracked { item: PackagedMethod::EndOfSession, promises: vec![tx] };
        let _ = self.sender.send(Message::Method { session: self.session, content });
        let _ = self.sender.send(Message::EndSession { session: self.session });
    }
}
