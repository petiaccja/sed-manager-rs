use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, OnceLock};
use tokio::sync::OnceCell as AsyncOnceCell;

use crate::device::Device;
use crate::messaging::com_id::{
    ComIdState, HandleComIdRequest, StackResetResponsePayload, StackResetStatus, VerifyComIdValidResponsePayload,
};
use crate::messaging::discovery::{Discovery, FeatureDescriptor, UnrecognizedDescriptor};
use crate::messaging::types::{List, MaxBytes32, NamedValue, SPRef};
use crate::messaging::value::Bytes;
use crate::rpc::args::{DecodeArgs, EncodeArgs};
use crate::rpc::{
    Error as RPCError, ErrorEvent as RPCErrorEvent, ErrorEventExt, MethodCall, MethodStatus, Properties, RPCSession,
};
use crate::serialization::{Deserialize, DeserializeBinary, InputStream};
use crate::specification::{invoker, method};

use super::session::Session;

pub struct TPer {
    device: Arc<dyn Device>,
    cached_discovery: OnceLock<Discovery>,
    cached_stack: AsyncOnceCell<Stack>,
    next_hsn: AtomicU32,
}

struct Stack {
    com_id: u16,
    com_id_ext: u16,
    rpc_session: RPCSession,
}

pub fn discover(device: &dyn Device) -> Result<Discovery, RPCError> {
    let data = device.security_recv(0x01, 0x0001_u16.to_be_bytes(), 4096).map_err(|err| err.while_receiving())?;
    let discovery = Discovery::from_bytes(data).map_err(|err| err.while_receiving())?;
    let descs: Vec<_> = discovery
        .descriptors
        .into_vec()
        .into_iter()
        .filter(|desc| match desc {
            FeatureDescriptor::Unrecognized(UnrecognizedDescriptor { feature_code: 0, length: 0, version: 0 }) => false,
            _ => true,
        })
        .collect();
    Ok(Discovery { descriptors: descs.into() })
}

impl TPer {
    pub fn new(device: Arc<dyn Device>) -> TPer {
        TPer {
            device: device.into(),
            cached_discovery: OnceLock::new(),
            cached_stack: AsyncOnceCell::new(),
            next_hsn: 1.into(),
        }
    }

    pub fn discovery(&self) -> Result<&Discovery, RPCError> {
        // - The device MAY allow level 0 discovery at any point in time.
        // - The data MUST either be truncated or padded by the device if the transfer length is not exact.
        match self.cached_discovery.get() {
            Some(discovery) => Ok(discovery),
            None => {
                let discovery = discover(self.device.as_ref())?;
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
                let ssc = discovery.descriptors.iter().find_map(|desc| desc.security_subsystem_class());
                let maybe_com_id = ssc.map(|ssc| ssc.base_com_id());
                let Some(com_id) = maybe_com_id else {
                    return Err(RPCErrorEvent::NotSupported.as_error());
                };
                let com_id_ext = 0x0000;
                let main_session = RPCSession::new(self.device.clone(), com_id, com_id_ext, Properties::default());

                Ok(Stack { com_id, com_id_ext, rpc_session: main_session })
            })
            .await
    }

    pub async fn com_id(&self) -> Result<u16, RPCError> {
        self.stack().await.map(|stack| stack.com_id)
    }

    pub async fn com_id_ext(&self) -> Result<u16, RPCError> {
        self.stack().await.map(|stack| stack.com_id_ext)
    }

    pub async fn active_properties(&self) -> Properties {
        if let Ok(stack) = self.stack().await {
            stack.rpc_session.get_properties().await
        } else {
            Properties::ASSUMED
        }
    }

    pub async fn verify_com_id(&self, com_id: u16, com_id_ext: u16) -> Result<ComIdState, RPCError> {
        let stack = self.stack().await?;
        let com_id_session = stack.rpc_session.get_com_session().await;
        let request = HandleComIdRequest::verify_com_id_valid(com_id, com_id_ext);
        let response = com_id_session.handle_request(request).await?;
        let mut stream = InputStream::from(response.payload.into_vec());
        match VerifyComIdValidResponsePayload::deserialize(&mut stream) {
            Ok(response) => Ok(response.com_id_state),
            Err(err) => Err(err.while_receiving()),
        }
    }

    pub async fn stack_reset(&self, com_id: u16, com_id_ext: u16) -> Result<StackResetStatus, RPCError> {
        let stack = self.stack().await?;
        let com_id_session = stack.rpc_session.get_com_session().await;
        let request = HandleComIdRequest::stack_reset(com_id, com_id_ext);
        let response = com_id_session.handle_request(request).await?;
        let mut stream = InputStream::from(response.payload.into_vec());
        match StackResetResponsePayload::deserialize(&mut stream) {
            Ok(response) => Ok(response.stack_reset_status),
            Err(err) => Err(err.while_receiving()),
        }
    }

    pub async fn properties(
        &self,
        host_properties: Option<List<NamedValue<MaxBytes32, u32>>>,
    ) -> Result<(List<NamedValue<MaxBytes32, u32>>, Option<List<NamedValue<MaxBytes32, u32>>>), RPCError> {
        let host_struct = Properties::from_list(host_properties.as_ref().unwrap_or(&List::new()));
        let call = MethodCall {
            invoking_id: invoker::SMUID,
            method_id: method::PROPERTIES,
            args: (host_properties,).encode_args(),
            status: MethodStatus::Success,
        };
        let stack = self.stack().await?;
        let control_session = stack.rpc_session.get_control_session().await;
        let result = control_session.call(call).await?;
        if result.status != MethodStatus::Success {
            return Err(result.status.while_receiving());
        }
        let (tper_properties, common_properties): (
            List<NamedValue<MaxBytes32, u32>>,
            Option<List<NamedValue<MaxBytes32, u32>>>,
        ) = result.args.decode_args().map_err(|err: MethodStatus| err.while_receiving())?;
        let tper_struct = Properties::from_list(&tper_properties);
        let stack_properties = Properties::common(&host_struct, &tper_struct);
        stack.rpc_session.set_properties(stack_properties).await;
        Ok((tper_properties, common_properties))
    }

    pub async fn start_session(&self, sp: SPRef) -> Result<Session, RPCError> {
        let hsn = self.next_hsn.fetch_add(1, Ordering::Relaxed);
        let call = MethodCall {
            invoking_id: invoker::SMUID,
            method_id: method::START_SESSION,
            args: (hsn, sp, 0u8).encode_args(),
            status: MethodStatus::Success,
        };
        let stack = self.stack().await?;
        let control_session = stack.rpc_session.get_control_session().await;
        let result = control_session.call(call).await?;
        if result.status != MethodStatus::Success {
            return Err(result.status.while_receiving());
        }
        let (hsn_sync, tsn_sync, _, _, _, _, _, _): (
            u32,
            u32,
            Option<Bytes>,
            Option<Bytes>,
            Option<Bytes>,
            Option<u32>,
            Option<u32>,
            Option<Bytes>,
        ) = result.args.decode_args().map_err(|err: MethodStatus| err.while_receiving())?;
        if hsn_sync != hsn {
            return Err(MethodStatus::InvalidParameter.while_receiving());
        }
        let sp_session = stack.rpc_session.open_sp_session(hsn, tsn_sync).await.expect("ensure HSN is unique");
        Ok(Session::new(sp_session))
    }
}
