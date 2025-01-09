use async_trait::async_trait;

use crate::messaging::packet::Packet;
use crate::rpc::error::Error;

#[async_trait]
pub trait PacketLayer: Send + Sync {
    async fn send(&self, packet: Packet) -> Result<(), Error>;
    async fn recv(&self) -> Result<Packet, Error>;
    async fn close(&self);
    async fn abort(&self);
}
