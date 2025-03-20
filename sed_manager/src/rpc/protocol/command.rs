use tokio::sync::{mpsc, oneshot};

use crate::messaging::com_id::{HandleComIdRequest, HandleComIdResponse};
use crate::messaging::discovery::Discovery;
use crate::rpc::{Error, PackagedMethod, Properties, SessionIdentifier};

use super::promise::Promise;

pub enum Command {
    OpenSession { id: SessionIdentifier, properties: Properties },
    CloseSession { id: SessionIdentifier },
    AbortSession { id: SessionIdentifier },
    CloseComSession,
    TryShutdown,
    Method { id: SessionIdentifier, request: Promise<PackagedMethod, PackagedMethod, Error> },
    ComId { request: Promise<HandleComIdRequest, HandleComIdResponse, Error> },
    Discover { request: Promise<(), Discovery, Error> },
}

#[derive(Clone)]
pub struct CommandSender {
    tx: mpsc::UnboundedSender<Command>,
}

impl CommandSender {
    pub fn new(tx: mpsc::UnboundedSender<Command>) -> Self {
        Self { tx }
    }

    pub fn open_session(&self, id: SessionIdentifier, properties: Properties) {
        let _ = self.tx.send(Command::OpenSession { id, properties });
    }

    pub fn close_session(&self, id: SessionIdentifier) {
        let _ = self.tx.send(Command::CloseSession { id });
    }

    pub fn abort_session(&self, id: SessionIdentifier) {
        let _ = self.tx.send(Command::AbortSession { id });
    }

    pub fn close_com_session(&self) {
        let _ = self.tx.send(Command::CloseComSession);
    }

    pub fn try_shutdown(&self) {
        let _ = self.tx.send(Command::TryShutdown);
    }

    pub fn enqueue_method(&self, id: SessionIdentifier, request: PackagedMethod) {
        let (tx, _rx) = oneshot::channel();
        let promise = Promise::new(request, vec![tx]);
        let _ = self.tx.send(Command::Method { id, request: promise });
    }

    pub async fn method(&self, id: SessionIdentifier, request: PackagedMethod) -> Result<PackagedMethod, Error> {
        let (tx, rx) = oneshot::channel();
        let promise = Promise::new(request, vec![tx]);
        let _ = self.tx.send(Command::Method { id, request: promise });
        match rx.await {
            Ok(response) => response,
            Err(_) => Err(Error::Closed),
        }
    }

    pub async fn com_id(&self, request: HandleComIdRequest) -> Result<HandleComIdResponse, Error> {
        let (tx, rx) = oneshot::channel();
        let promise = Promise::new(request, vec![tx]);
        let _ = self.tx.send(Command::ComId { request: promise });
        match rx.await {
            Ok(response) => response,
            Err(_) => Err(Error::Closed),
        }
    }

    pub async fn discover(&self) -> Result<Discovery, Error> {
        let (tx, rx) = oneshot::channel();
        let promise = Promise::new((), vec![tx]);
        let _ = self.tx.send(Command::Discover { request: promise });
        match rx.await {
            Ok(response) => response,
            Err(_) => Err(Error::Closed),
        }
    }
}
