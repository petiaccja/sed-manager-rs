use crate::serialization::{Deserialize, Serialize};

use super::error::Error;
use super::method::{MethodCall, MethodResult};
use super::protocol::{ComPacketLayer, InterfaceLayer, MethodLayer, PackagedMethod, PacketLayer, SessionControl};

pub struct Engine {}

pub struct CommunicationSession {}

pub struct ManagementSession {}

pub struct ConfigSession {}

impl Engine {
    pub fn get_communication_session(&self) -> &CommunicationSession {
        todo!()
    }

    pub fn get_management_session(&self) -> &ManagementSession {
        todo!()
    }

    fn create_config_session(&self) -> ConfigSession {
        todo!()
    }
}

impl CommunicationSession {
    // pub async fn handle_com_id<Result: Deserialize, Request: Serialize>(request: Request) {
    //     todo!()
    // }
}

impl ManagementSession {
    pub async fn call(method_call: MethodCall) -> Result<MethodCall, Error> {
        todo!()
    }
}

impl ConfigSession {
    pub async fn call(method_call: MethodCall) -> Result<MethodResult, Error> {
        todo!()
    }

    pub async fn close() -> Result<(), Error> {
        todo!()
    }
}
