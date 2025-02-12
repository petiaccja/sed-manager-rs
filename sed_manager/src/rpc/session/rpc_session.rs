use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;

use tokio::sync::Mutex;

use super::com_session::ComSession;
use super::control_session::ControlSession;
use super::sp_session::SPSession;
use crate::device::Device;
use crate::rpc::properties::Properties;
use crate::rpc::protocol::{message_loop, Message, RPCStack};

pub struct RPCSession {
    properties: Mutex<Properties>,
    message_loop: Option<JoinHandle<()>>,
    messaging: Option<Messaging>,
}

struct Messaging {
    sender: mpsc::Sender<Message>,
    com_session: ComSession,
    control_session: ControlSession,
}

impl RPCSession {
    pub fn new(device: Arc<dyn Device>, com_id: u16, com_id_ext: u16, properties: Properties) -> Self {
        let (sender, receiver) = mpsc::channel();
        let stack = RPCStack::new(com_id, com_id_ext);
        Self {
            properties: properties.into(),
            message_loop: Some(std::thread::spawn(move || message_loop(receiver, device, stack))),
            messaging: Some(Messaging {
                sender: sender.clone(),
                com_session: ComSession::new(sender.clone()),
                control_session: ControlSession::new(sender.clone()),
            }),
        }
    }

    pub async fn set_properties(&self, properties: Properties) {
        *self.properties.lock().await = properties;
    }

    pub async fn get_properties(&self) -> Properties {
        self.properties.lock().await.clone()
    }

    pub async fn get_com_session(&self) -> &ComSession {
        &self.messaging.as_ref().unwrap().com_session
    }

    pub async fn get_control_session(&self) -> &ControlSession {
        &self.messaging.as_ref().unwrap().control_session
    }

    pub async fn open_sp_session(&self, host_session_number: u32, tper_session_number: u32) -> Option<SPSession> {
        Some(SPSession::new(
            host_session_number,
            tper_session_number,
            self.properties.lock().await.clone(),
            self.messaging.as_ref().unwrap().sender.clone(),
        ))
    }
}

impl Drop for RPCSession {
    fn drop(&mut self) {
        drop(self.messaging.take().unwrap());
        drop(self.message_loop.take().unwrap().join());
    }
}
