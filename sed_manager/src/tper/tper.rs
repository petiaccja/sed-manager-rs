use std::cell::OnceCell;

use crate::device::Device;
use crate::messaging::com_id::ComIdState;
use crate::messaging::discovery::{Discovery, FeatureCode};
use crate::rpc::{ComIdSession, Error as RPCError, MainSession};
use crate::serialization::{Deserialize, InputStream};

pub struct TPer {
    device: Box<dyn Device>,
    cached_discovery: OnceCell<Discovery>,
}

struct Stack {
    com_id: u16,
    com_id_ext: u16,
    main_session: MainSession,
    com_id_session: ComIdSession,
}

impl TPer {
    pub fn new(device: Box<dyn Device>) -> TPer {
        TPer { device: device, cached_discovery: OnceCell::new() }
    }

    pub fn take(self) -> Box<dyn Device> {
        self.device
    }

    pub fn discovery(&self) -> Result<&Discovery, RPCError> {
        // - The device MAY allow level 0 discovery at any point in time.
        // - The data MUST either be truncated or padded by the device if the transfer length is not exact.
        match self.cached_discovery.get() {
            Some(discovery) => Ok(discovery),
            None => {
                let data = match self.device.security_recv(0x01, 0x0001_u16.to_be_bytes(), 4096) {
                    Ok(data) => data,
                    Err(err) => return Err(RPCError::SecurityReceiveFailed(err)),
                };
                let mut stream = InputStream::from(data);
                let discovery = match Discovery::deserialize(&mut stream) {
                    Ok(discovery) => discovery,
                    Err(err) => return Err(RPCError::SerializationFailed(err)),
                };
                Ok(self.cached_discovery.get_or_init(|| discovery))
            }
        }
    }

    async fn stack(&self) -> Result<&Stack, RPCError> {
        let discovery = self.discovery()?;
        discovery.get(FeatureCode::Enterprise);
        todo!()
    }

    pub fn com_id(&self) -> u16 {
        todo!()
    }

    pub fn com_id_ext(&self) -> u16 {
        todo!()
    }

    pub async fn verify_com_id(&self, com_id: u16, com_id_ext: u16) -> Result<ComIdState, RPCError> {
        todo!()
    }

    pub async fn stack_reset(&self, com_id: u16, com_id_ext: u16) -> Result<(), RPCError> {
        todo!()
    }
}
