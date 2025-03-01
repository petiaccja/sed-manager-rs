use tokio::sync::Mutex;

use crate::messaging::value::Bytes;
use crate::rpc::args::{DecodeArgs as _, EncodeArgs as _};
use crate::rpc::{
    CommandSender, Error as RPCError, ErrorEvent as RPCErrorEvent, ErrorEventExt as _, MethodCall, MethodStatus,
    PackagedMethod, Properties, SessionIdentifier,
};
use crate::spec::basic_types::{List, NamedValue};
use crate::spec::column_types::{AuthorityRef, MaxBytes32, SPRef};
use crate::spec::{invoking_id::*, sm_method_id::*};

const CONTROL_SESSION_ID: SessionIdentifier = SessionIdentifier { hsn: 0, tsn: 0 };

pub struct ControlSession {
    sender: CommandSender,
    mutex: Mutex<()>,
}

#[allow(unused)] // Features that use some fields are not implemented.
pub struct SyncSession {
    pub hsn: u32,
    pub tsn: u32,
    pub sp_challenge: Option<Bytes>,
    pub sp_exchange_cert: Option<Bytes>,
    pub sp_signing_cert: Option<Bytes>,
    pub trans_timeout: Option<u32>,
    pub initial_credit: Option<u32>,
    pub signed_hash: Option<Bytes>,
}

impl ControlSession {
    pub fn new(sender: CommandSender) -> Self {
        let _ = sender.open_session(CONTROL_SESSION_ID, Properties::ASSUMED);
        Self { sender, mutex: ().into() }
    }

    pub async fn do_method_call(&self, method: MethodCall) -> Result<MethodCall, RPCError> {
        let _guard = self.mutex.lock().await;
        let result = self.sender.method(CONTROL_SESSION_ID, PackagedMethod::Call(method)).await;
        match result {
            Ok(PackagedMethod::Call(call)) => return Ok(call),
            Ok(_) => return Err(RPCErrorEvent::MethodCallExpected.while_receiving()),
            Err(error) => Err(error),
        }
    }
}

impl Drop for ControlSession {
    fn drop(&mut self) {
        let _ = self.sender.close_session(CONTROL_SESSION_ID);
    }
}

impl ControlSession {
    pub async fn properties(
        &self,
        host_properties: Option<List<NamedValue<MaxBytes32, u32>>>,
    ) -> Result<(List<NamedValue<MaxBytes32, u32>>, Option<List<NamedValue<MaxBytes32, u32>>>), RPCError> {
        let call = MethodCall::new_success(SESSION_MANAGER, PROPERTIES, (host_properties,).encode_args());
        let result = self.do_method_call(call).await?.take_args()?;
        let (tper_capabilities, tper_properties) =
            result.decode_args().map_err(|err: MethodStatus| err.while_receiving())?;
        Ok((tper_capabilities, tper_properties))
    }

    pub async fn start_session(
        &self,
        hsn: u32,
        sp: SPRef,
        write: bool,
        host_challenge: Option<&[u8]>,
        host_exchange_authority: Option<AuthorityRef>,
        host_exchange_cert: Option<&[u8]>,
        host_signing_authority: Option<AuthorityRef>,
        host_signing_cert: Option<&[u8]>,
        session_timeout: Option<u32>,
        trans_timeout: Option<u32>,
        initial_credit: Option<u32>,
        signed_hash: Option<&[u8]>,
    ) -> Result<SyncSession, RPCError> {
        let args = (
            hsn,
            sp,
            write,
            host_challenge,
            host_exchange_authority,
            host_exchange_cert,
            host_signing_authority,
            host_signing_cert,
            session_timeout,
            trans_timeout,
            initial_credit,
            signed_hash,
        )
            .encode_args();
        let call = MethodCall::new_success(SESSION_MANAGER, START_SESSION, args);
        let result = self.do_method_call(call).await?.take_args()?;
        let (hsn, tsn, sp_challenge, sp_exchange_cert, sp_signing_cert, trans_timeout, initial_credit, signed_hash) =
            result.decode_args().map_err(|err: MethodStatus| err.while_receiving())?;
        Ok(SyncSession {
            hsn,
            tsn,
            sp_challenge,
            sp_exchange_cert,
            sp_signing_cert,
            trans_timeout,
            initial_credit,
            signed_hash,
        })
    }
}
