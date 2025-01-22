use std::sync::Arc;

use tokio::sync::{Mutex, OnceCell};

use super::com_session::ComSession;
use super::control_session::ControlSession;
use super::sp_session::SPSession;
use crate::async_finalize::{async_finalize, sync_finalize, AsyncFinalize};
use crate::device::Device;
use crate::rpc::properties::Properties;
use crate::rpc::protocol::{
    ComPacketBundler, InterfaceLayer, MethodCaller, PacketLayer, SessionRouter, SynchronousHost,
};

pub struct RPCSession {
    interface_layer: Arc<dyn InterfaceLayer>,
    session_router: Arc<SessionRouter>,
    com_session: OnceCell<ComSession>,
    control_session: OnceCell<ControlSession>,
    properties: Mutex<Properties>,
}

impl RPCSession {
    pub fn new(device: Arc<dyn Device>, com_id: u16, com_id_ext: u16, properties: Properties) -> Self {
        let interface_layer = Arc::new(SynchronousHost::new(device, com_id, properties.clone()));
        let com_packet_layer = ComPacketBundler::new(com_id, com_id_ext, interface_layer.clone(), properties.clone());
        let session_rounter = SessionRouter::new(Box::new(com_packet_layer));
        Self {
            interface_layer,
            session_router: session_rounter.into(),
            com_session: OnceCell::new(),
            control_session: OnceCell::new(),
            properties: properties.into(),
        }
    }

    pub async fn set_properties(&self, properties: Properties) {
        *self.properties.lock().await = properties;
    }

    pub async fn get_properties(&self) -> Properties {
        self.properties.lock().await.clone()
    }

    pub async fn get_com_session(&self) -> &ComSession {
        self.com_session.get_or_init(|| async { ComSession::new(self.interface_layer.clone()) }).await
    }

    pub async fn get_control_session(&self) -> &ControlSession {
        self.control_session
            .get_or_init(|| async {
                let layer = self.create_session(0, 0).await.unwrap();
                ControlSession::new(layer)
            })
            .await
    }

    pub async fn open_sp_session(&self, host_session_number: u32, tper_session_number: u32) -> Option<SPSession> {
        self.create_session(host_session_number, tper_session_number)
            .await
            .map(|layer| SPSession::new(layer))
    }

    async fn create_session(&self, host_sn: u32, tper_sn: u32) -> Option<MethodCaller> {
        let Some(session_endpoint) = self.session_router.clone().open(host_sn, tper_sn).await else {
            return None;
        };
        let layer: Box<dyn PacketLayer> = Box::new(session_endpoint);

        Some(MethodCaller::new(layer, self.properties.lock().await.clone()))
    }
}

impl AsyncFinalize for RPCSession {
    async fn finalize(&mut self) {
        if let Some(com_session) = self.com_session.get_mut() {
            async_finalize(com_session).await;
        }
        if let Some(control_session) = self.control_session.get_mut() {
            async_finalize(control_session).await;
        }
    }
}

impl Drop for RPCSession {
    fn drop(&mut self) {
        sync_finalize(self);
    }
}
