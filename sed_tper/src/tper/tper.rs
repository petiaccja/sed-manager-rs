use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use sed_async_runtime::spawn;
use sed_device::Device;
use sed_packet::discovery::Discovery;
use sed_packet::{
    MaxBytes,
    com_id::{ComIdRequest, ComIdResponsePayload, ComIdState, StackResetStatus},
};
use sed_spec::objects::{AuthorityRef, SecurityProviderRef};
use sorbit::ser_de::FromBytes;
use tracing::instrument;

use crate::{
    Session,
    error::Error,
    protocol::{Controller, Protocol},
};

#[derive(Debug)]
pub struct TPer {
    controller: Controller,
    host_session_id: AtomicU32,
}

impl TPer {
    /// Connect to a device on the specified ComID and ComID extension.
    ///
    /// The ComID pair has to either be a static ComID or dynamically allocated
    /// prior to calling this function.
    ///
    /// To handle communication with the device, and a protocol is spawned
    /// on the current async runtime. If you shut down that runtime, the
    /// protocol will be killed, all subsequent requests will time out, and
    /// you'll likely need to do a stack reset to get the device's communication
    /// stack synchronized again.
    #[instrument(level = "info")]
    pub async fn connect(com_id: u16, com_id_ext: u16, device: Arc<dyn Device>) -> Self {
        let (protocol, controller) = Protocol::new(com_id, com_id_ext, device);
        spawn(protocol.run());
        Self { controller, host_session_id: 1.into() }
    }

    #[instrument(level = "info", ret, err)]
    pub async fn discover(device: &dyn Device) -> Result<Discovery, Error> {
        let bytes = device.security_recv(0x01, 0x0001_u16.to_be_bytes(), 4096)?;
        Discovery::from_bytes(&bytes).map_err(|error| Error::InvalidDiscovery(error))
    }

    #[instrument(level = "info", ret, err)]
    pub async fn verify_com_id_valid(&self, com_id: u16, com_id_ext: u16) -> Result<ComIdState, Error> {
        use ComIdResponsePayload::*;

        let request = ComIdRequest::verify_com_id_valid(com_id, com_id_ext);
        let response = self.controller.com_id_request(request).await.map_err(|_| Error::Closed)??;
        match response.payload {
            Verify { com_id_state, .. } => Ok(com_id_state),
            _ => Err(Error::TimedOut),
        }
    }

    #[instrument(level = "info", ret, err)]
    pub async fn stack_reset(&self, com_id: u16, com_id_ext: u16) -> Result<(), Error> {
        use ComIdResponsePayload::*;

        let request = ComIdRequest::stack_reset(com_id, com_id_ext);
        let response = self.controller.com_id_request(request).await.map_err(|_| Error::Closed)??;
        match response.payload {
            // TODO: We should loop to avoid this scenario, probably inside the Protocol.
            StackReset { available_data_length: 0, .. } => Ok(()),
            StackReset { status: StackResetStatus::Success, available_data_length: 1.. } => Ok(()),
            StackReset { status: StackResetStatus::Failure, available_data_length: 1.. } => {
                Err(Error::StackResetFailed)
            }
            NoResponseAvailable { .. } => Err(Error::TimedOut),
            Verify { .. } => Err(Error::TimedOut),
        }
    }

    #[instrument(level = "debug", ret, err)]
    pub async fn start_session(
        &self,
        sp: SecurityProviderRef,
        authority: Option<AuthorityRef>,
        password: Option<MaxBytes<32>>,
    ) -> Result<Session, Error> {
        let host_session_number = self.host_session_id.fetch_add(1, Ordering::Relaxed);
        Session::start(self.controller.clone(), host_session_number, sp, authority, password).await
    }
}
