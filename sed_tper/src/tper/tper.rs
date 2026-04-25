use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};

use sed_async_runtime::spawn;
use sed_device::Device;
use sed_packet::{
    MaxBytes,
    com_id::{ComIdState, HandleComIdRequest, HandleComIdResponseParams, StackResetStatus},
    token::{FromTokens as _, ToTokens as _},
};
use sed_spec::{
    methods::{MethodCall, MethodStatus, StartSession, SyncSession},
    objects::{AuthorityRef, SpRef},
    preconfig::core::shared::{invoking_id::SESSION_MANAGER, sm_method_id::START_SESSION},
};
use tracing::instrument;

use crate::{
    Session,
    error::Error,
    protocol::{Controller, Protocol, SessionId},
};

#[derive(Debug)]
pub struct TPer {
    controller: Controller,
    host_session_id: AtomicU32,
}

impl TPer {
    pub async fn new(com_id: u16, com_id_ext: u16, device: Arc<dyn Device>) -> Self {
        let (protocol, controller) = Protocol::new(com_id, com_id_ext, device);
        spawn(protocol.run());
        Self { controller, host_session_id: 1.into() }
    }

    #[instrument(level = "info")]
    pub async fn stack_reset(&self, com_id: u16, com_id_ext: u16) -> Result<(), Error> {
        use HandleComIdResponseParams::*;

        let request = HandleComIdRequest::stack_reset(com_id, com_id_ext);
        let response = self.controller.com_id_request(request).await.map_err(|_| Error::Closed)??;
        match response.params {
            StackReset { status: StackResetStatus::Success | StackResetStatus::Pending, .. } => Ok(()),
            StackReset { status: StackResetStatus::Failure, .. } => Err(Error::StackResetFailed),
            NoResponseAvailable { .. } => Err(Error::TimedOut),
            VerifyComIdValid { .. } => Err(Error::TimedOut),
        }
    }

    #[instrument(level = "info")]
    pub async fn verify_com_id_valid(&self, com_id: u16, com_id_ext: u16) -> Result<ComIdState, Error> {
        use HandleComIdResponseParams::*;

        let request = HandleComIdRequest::verify_com_id_valid(com_id, com_id_ext);
        let response = self.controller.com_id_request(request).await.map_err(|_| Error::Closed)??;
        match response.params {
            VerifyComIdValid { com_id_state, .. } => Ok(com_id_state),
            _ => Err(Error::TimedOut),
        }
    }

    #[instrument(level = "debug")]
    pub async fn start_session(
        &self,
        sp: SpRef,
        authority: Option<AuthorityRef>,
        password: Option<MaxBytes<32>>,
    ) -> Result<Session, Error> {
        let host_session_id = self.host_session_id.fetch_add(1, Ordering::Relaxed);
        let call = MethodCall {
            invoking_id: SESSION_MANAGER,
            method_id: START_SESSION,
            parameters: StartSession {
                host_session_id,
                spid: sp,
                write: true,
                host_challenge: password,
                host_exchange_authority: None,
                host_exchange_cert: None,
                host_signing_authority: authority,
                host_signing_cert: None,
                session_timeout: None,
                trans_timeout: None,
                initial_credit: None,
                signed_hash: None,
            },
            status: MethodStatus::Success,
        };
        let result_tokens = self
            .controller
            .call(SessionId::MANAGEMENT, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        let result = MethodCall::<SyncSession>::from_tokens(&result_tokens)?;
        if result.status == MethodStatus::Success {
            let tper_session_id = result.parameters.sp_session_id;
            let session_id = SessionId { hsn: host_session_id, tsn: tper_session_id };
            Ok(Session::new(session_id, self.controller.clone()))
        } else {
            Err(result.status.into())
        }
    }
}
