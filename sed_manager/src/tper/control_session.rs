use tokio::sync::{oneshot, Mutex};

use crate::messaging::types::{List, MaxBytes32, NamedValue, SPRef};
use crate::messaging::value::Bytes;
use crate::rpc::args::{DecodeArgs as _, EncodeArgs as _};
use crate::rpc::{
    Error as RPCError, ErrorEvent as RPCErrorEvent, ErrorEventExt as _, Message, MessageSender, MethodCall,
    MethodStatus, PackagedMethod, Properties, SessionIdentifier, Tracked,
};
use crate::specification::{invoker, method};

const CONTROL_SESSION_ID: SessionIdentifier = SessionIdentifier { hsn: 0, tsn: 0 };

pub struct ControlSession {
    sender: MessageSender,
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
    pub fn new(sender: MessageSender) -> Self {
        let _ = sender.send(Message::StartSession { session: CONTROL_SESSION_ID, properties: Properties::ASSUMED });
        Self { sender, mutex: ().into() }
    }

    pub async fn do_method_call(&self, method: MethodCall) -> Result<MethodCall, RPCError> {
        let (tx, rx) = oneshot::channel();
        let content = Tracked { item: PackagedMethod::Call(method), promises: vec![tx] };
        let _guard = self.mutex.lock().await;
        let _ = self.sender.send(Message::Method { session: CONTROL_SESSION_ID, content });
        let result = match rx.await {
            Ok(result) => result,
            Err(_) => Err(RPCErrorEvent::Closed.while_receiving()),
        };
        match result {
            Ok(PackagedMethod::Call(call)) => return Ok(call),
            Ok(_) => return Err(RPCErrorEvent::MethodCallExpected.while_receiving()),
            Err(error) => Err(error),
        }
    }
}

impl Drop for ControlSession {
    fn drop(&mut self) {
        let _ = self.sender.send(Message::EndSession { session: CONTROL_SESSION_ID });
    }
}

impl ControlSession {
    pub async fn properties(
        &self,
        host_properties: Option<List<NamedValue<MaxBytes32, u32>>>,
    ) -> Result<(List<NamedValue<MaxBytes32, u32>>, Option<List<NamedValue<MaxBytes32, u32>>>), RPCError> {
        let call = MethodCall::new_success(invoker::SMUID, method::PROPERTIES, (host_properties,).encode_args());
        let result = self.do_method_call(call).await?.take_args()?;
        let (tper_capabilities, tper_properties) =
            result.decode_args().map_err(|err: MethodStatus| err.while_receiving())?;
        Ok((tper_capabilities, tper_properties))
    }

    pub async fn start_session(&self, sp: SPRef, hsn: u32) -> Result<SyncSession, RPCError> {
        let call = MethodCall::new_success(invoker::SMUID, method::START_SESSION, (hsn, sp, 0u8).encode_args());
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
