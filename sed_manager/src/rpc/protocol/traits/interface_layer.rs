use async_trait::async_trait;

use crate::messaging::com_id::{HandleComIdRequest, HandleComIdResponse};
use crate::messaging::packet::ComPacket;
use crate::rpc::error::Error;

#[async_trait]
pub trait InterfaceLayer: Sync + Send {
    async fn send_handle_com_id(&self, handle_com_id: HandleComIdRequest) -> Result<(), Error>;
    async fn recv_handle_com_id(&self) -> Result<HandleComIdResponse, Error>;
    async fn send_com_packet(&self, com_packet: ComPacket) -> Result<(), Error>;
    async fn recv_com_packet(&self) -> Result<ComPacket, Error>;
    async fn close(&self);
    async fn abort(&self);
}
