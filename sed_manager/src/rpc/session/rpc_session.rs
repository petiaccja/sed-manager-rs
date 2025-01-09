use std::sync::Arc;

use tokio::sync::OnceCell;

use super::com_session::ComSession;
use super::management_session::ManagementSession;
use super::sp_session::SPSession;
use crate::device::Device;
use crate::rpc::properties::Properties;
use crate::rpc::protocol::{ComPacketLayer, InterfaceLayer, MethodLayer, MultiplexerHub, PacketLayer, SyncHostLayer};

pub struct RPCSession {
    interface_layer: Arc<dyn InterfaceLayer>,
    mux_hub: MultiplexerHub,
    com_session: OnceCell<ComSession>,
    mgmt_session: OnceCell<ManagementSession>,
    properties: Properties,
}

impl RPCSession {
    pub fn new(device: Arc<dyn Device>, com_id: u16, com_id_ext: u16, properties: Properties) -> Self {
        let interface_layer = Arc::new(SyncHostLayer::new(device, com_id, properties.clone()));
        let com_packet_layer = ComPacketLayer::new(com_id, com_id_ext, interface_layer.clone(), properties.clone());
        let multiplexer_hub = MultiplexerHub::new(Box::new(com_packet_layer));
        Self {
            interface_layer,
            mux_hub: multiplexer_hub,
            com_session: OnceCell::new(),
            mgmt_session: OnceCell::new(),
            properties,
        }
    }

    pub async fn get_com_session(&self) -> &ComSession {
        self.com_session.get_or_init(|| async { ComSession::new(self.interface_layer.clone()) }).await
    }

    pub async fn get_management_session(&self) -> &ManagementSession {
        self.mgmt_session
            .get_or_init(|| async {
                let layer = self.create_session(0, 0).await.unwrap();
                ManagementSession::new(layer)
            })
            .await
    }

    pub async fn create_sp_session(&self, host_session_number: u32, tper_session_number: u32) -> Option<SPSession> {
        self.create_session(host_session_number, tper_session_number)
            .await
            .map(|layer| SPSession::new(layer))
    }

    async fn create_session(&self, host_sn: u32, tper_sn: u32) -> Option<MethodLayer> {
        // Multiplexer.
        let Some(mux_session) = self.mux_hub.create_session(host_sn, tper_sn).await else {
            return None;
        };
        let layer: Box<dyn PacketLayer> = Box::new(mux_session);

        Some(MethodLayer::new(layer, self.properties.clone()))
    }
}
