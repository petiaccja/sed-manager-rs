use async_trait::async_trait;
use std::sync::Arc;

use crate::{messaging::packet::Packet, rpc::Error};

use super::{session_router::SessionRouter, PacketLayer};

pub struct SessionEndpoint {
    host_session_number: u32,
    tper_session_number: u32,
    router: Arc<SessionRouter>,
}

impl SessionEndpoint {
    pub fn new(host_session_number: u32, tper_session_number: u32, router: Arc<SessionRouter>) -> Self {
        Self { host_session_number, tper_session_number, router }
    }
}

#[async_trait]
impl PacketLayer for SessionEndpoint {
    async fn send(&self, packet: Packet) -> Result<(), Error> {
        let packet = Packet {
            tper_session_number: self.tper_session_number,
            host_session_number: self.host_session_number,
            ..packet
        };
        self.router.send(packet).await
    }

    async fn recv(&self) -> Result<Packet, Error> {
        self.router.recv(self.host_session_number, self.tper_session_number).await
    }

    async fn close(&self) {
        self.router.close(self.host_session_number, self.tper_session_number).await;
    }

    async fn abort(&self) {
        self.router.close(self.host_session_number, self.tper_session_number).await;
    }
}
