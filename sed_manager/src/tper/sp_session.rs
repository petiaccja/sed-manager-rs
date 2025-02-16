use tokio::sync::oneshot;

use crate::messaging::types::{AuthorityRef, CellBlock, List, NamedValue, ObjectReference, TableReference};
use crate::messaging::uid::UID;
use crate::messaging::value::{Bytes, Value};
use crate::rpc::args::{DecodeArgs, EncodeArgs};
use crate::rpc::{
    Error as RPCError, ErrorAction as RPCErrorAction, ErrorEvent as RPCErrorEvent, ErrorEventExt as _, Message,
    MessageSender, MethodCall, MethodResult, MethodStatus, PackagedMethod, Properties, SessionIdentifier, Tracked,
};
use crate::specification::{invoker, method};

pub struct SPSession {
    session: SessionIdentifier,
    sender: MessageSender,
}

impl SPSession {
    pub fn new(session: SessionIdentifier, sender: MessageSender, properties: Properties) -> Self {
        let _ = sender.send(Message::StartSession { session, properties });
        Self { session, sender }
    }

    async fn do_packaged_method(&self, packaged_method: PackagedMethod) -> Result<PackagedMethod, RPCError> {
        let (tx, rx) = oneshot::channel();
        let content = Tracked { item: packaged_method, promises: vec![tx] };
        let _ = self.sender.send(Message::Method { session: self.session, content });
        let result = match rx.await {
            Ok(result) => result,
            Err(_) => Err(RPCErrorEvent::Closed.while_receiving()),
        };
        let error = match result {
            Ok(package_method) => return Ok(package_method),
            Err(error) => error,
        };
        if is_cause_for_abort(&error) {
            self.do_abort();
        }
        Err(error)
    }

    async fn do_method_call(&self, call: MethodCall) -> Result<MethodResult, RPCError> {
        let result = self.do_packaged_method(PackagedMethod::Call(call)).await?;
        match result {
            PackagedMethod::Result(result) => Ok(result),
            _ => Err(self.do_abort()),
        }
    }

    async fn do_end_of_session(&self) -> Result<(), RPCError> {
        let result = self.do_packaged_method(PackagedMethod::EndOfSession).await?;
        match result {
            PackagedMethod::EndOfSession => Ok(()),
            _ => Err(self.do_abort()),
        }
    }

    fn do_abort(&self) -> RPCError {
        let _ = self.sender.send(Message::AbortSession { session: self.session });
        RPCErrorEvent::Aborted.while_receiving()
    }
}

impl Drop for SPSession {
    fn drop(&mut self) {
        let (tx, _rx) = oneshot::channel();
        let content = Tracked { item: PackagedMethod::EndOfSession, promises: vec![tx] };
        let _ = self.sender.send(Message::Method { session: self.session, content });
        let _ = self.sender.send(Message::EndSession { session: self.session });
    }
}

impl SPSession {
    pub async fn end_session(self) -> Result<(), RPCError> {
        self.do_end_of_session().await
    }

    pub async fn authenticate(&self, authority: AuthorityRef, proof: Option<Bytes>) -> Result<bool, RPCError> {
        let call = MethodCall::new_success(invoker::THIS_SP, method::AUTHENTICATE, (authority, proof).encode_args());
        let results = self.do_method_call(call).await?.take_results()?;
        // I'll assume the result is encoded without the typeOr{} NVP.
        // Not clear in spec, no official examples.
        let (success,) = results.decode_args().map_err(|err: MethodStatus| err.while_receiving())?;
        Ok(success)
    }

    pub async fn get<T: TryFrom<Value>>(&self, object: UID, column: u16) -> Result<T, RPCError> {
        let call = MethodCall::new_success(object, method::GET, (CellBlock::object(column..=column),).encode_args());
        let results = self.do_method_call(call).await?.take_results()?;
        // According to the TCG examples, result is encoded without typeOr{} name-value pair.
        let (mut column_values,): (Vec<Value>,) = results.decode_args()?;
        if let Some(value) = column_values.pop() {
            if let Ok(nvp) = NamedValue::<u64, T>::try_from(value) {
                Ok(nvp.value)
            } else {
                Err(RPCErrorEvent::InvalidColumnType.while_receiving())
            }
        } else {
            Err(MethodStatus::NotAuthorized.while_receiving())
        }
    }

    pub async fn set<T: Into<Value>>(&self, object: UID, column: u16, value: T) -> Result<(), RPCError> {
        let where_ = Option::<ObjectReference>::None; // According to the TCG examples, encoded without typeOr{} name-value pair.
        let values = Some(vec![Value::from(NamedValue { name: column, value })]); // According to the TCG examples, encoded without typeOr{} name-value pair.
        let call = MethodCall::new_success(object, method::SET, (where_, values).encode_args());
        let _ = self.do_method_call(call).await?.take_results()?; // `Set` returns nothing.
        Ok(())
    }

    pub async fn next(
        &self,
        table: TableReference,
        first: Option<ObjectReference>,
        count: Option<u64>,
    ) -> Result<List<ObjectReference>, RPCError> {
        let call = MethodCall::new_success(table.into(), method::NEXT, (first, count).encode_args());
        let results = self.do_method_call(call).await?.take_results()?;
        let (objects,) = results.decode_args().map_err(|err: MethodStatus| err.while_receiving())?;
        Ok(objects)
    }

    pub async fn get_acl(&self) {
        todo!()
    }

    pub async fn gen_key(&self) {
        todo!()
    }

    pub async fn revert(&self) {
        todo!()
    }

    pub async fn revert_sp(&self) {
        todo!()
    }

    pub async fn activate(&self) {
        todo!()
    }

    pub async fn random(&self) {
        todo!()
    }
}

fn is_cause_for_abort(error: &RPCError) -> bool {
    let is_event_for_abort = match error.event {
        RPCErrorEvent::Aborted => false,
        RPCErrorEvent::Closed => false,
        RPCErrorEvent::MethodFailed(_) => false,
        _ => true,
    };
    is_event_for_abort && error.action == RPCErrorAction::Receive
}
