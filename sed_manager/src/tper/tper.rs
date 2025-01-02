use std::sync::{Arc, OnceLock};
use tokio::sync::OnceCell as AsyncOnceCell;

use crate::device::Device;
use crate::messaging::com_id::{
    ComIdState, HandleComIdRequest, StackResetResponsePayload, StackResetStatus, VerifyComIdValidResponsePayload,
};
use crate::messaging::discovery::{Discovery, SSCDescriptor};
use crate::rpc::{Error as RPCError, Properties, TPerSession};
use crate::serialization::{Deserialize, InputStream};

pub struct TPer {
    device: Arc<dyn Device>,
    cached_discovery: OnceLock<Discovery>,
    cached_stack: AsyncOnceCell<Stack>,
}

struct Stack {
    com_id: u16,
    com_id_ext: u16,
    main_session: TPerSession,
}

impl TPer {
    pub fn new(device: Arc<dyn Device>) -> TPer {
        TPer { device: device.into(), cached_discovery: OnceLock::new(), cached_stack: AsyncOnceCell::new() }
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
                // Performance problem:
                // The above code and IF-RECV may be invoked concurrently on multiple threads.
                // This will work correctly, but may be wasteful with performance.
                // The solution is to use `get_or_try_init` which is as of yet only nightly.
                Ok(self.cached_discovery.get_or_init(|| discovery))
            }
        }
    }

    async fn stack(&self) -> Result<&Stack, RPCError> {
        self.cached_stack
            .get_or_try_init(|| async {
                let discovery = self.discovery()?;
                let ssc_desc = discovery.descriptors.iter().find_map(|desc| desc.ssc_desc());
                let Some(SSCDescriptor { base_com_id, num_com_ids: _ }) = ssc_desc else {
                    return Err(RPCError::Unsupported);
                };
                let com_id = base_com_id;
                let com_id_ext = 0x0000;
                let main_session = TPerSession::new(self.device.clone(), com_id, com_id_ext, Properties::default());

                Ok(Stack { com_id, com_id_ext, main_session })
            })
            .await
    }

    pub async fn com_id(&self) -> Result<u16, RPCError> {
        self.stack().await.map(|stack| stack.com_id)
    }

    pub async fn com_id_ext(&self) -> Result<u16, RPCError> {
        self.stack().await.map(|stack| stack.com_id_ext)
    }

    pub async fn verify_com_id(&self, com_id: u16, com_id_ext: u16) -> Result<ComIdState, RPCError> {
        let stack = self.stack().await?;
        let com_id_session = stack.main_session.get_com_id_session().await;
        let request = HandleComIdRequest::verify_com_id_valid(com_id, com_id_ext);
        let response = com_id_session.handle_request(request).await?;
        let mut stream = InputStream::from(response.payload.into_vec());
        match VerifyComIdValidResponsePayload::deserialize(&mut stream) {
            Ok(response) => Ok(response.com_id_state),
            Err(err) => Err(RPCError::SerializationFailed(err)),
        }
    }

    pub async fn stack_reset(&self, com_id: u16, com_id_ext: u16) -> Result<StackResetStatus, RPCError> {
        let stack = self.stack().await?;
        let com_id_session = stack.main_session.get_com_id_session().await;
        let request = HandleComIdRequest::stack_reset(com_id, com_id_ext);
        let response = com_id_session.handle_request(request).await?;
        let mut stream = InputStream::from(response.payload.into_vec());
        match StackResetResponsePayload::deserialize(&mut stream) {
            Ok(response) => Ok(response.stack_reset_status),
            Err(err) => Err(RPCError::SerializationFailed(err)),
        }
    }
}
