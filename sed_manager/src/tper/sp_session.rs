//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::ops::RangeBounds;

use crate::messaging::uid::{TableUID, UID};
use crate::messaging::value::{Bytes, Value};
use crate::rpc::args::{IntoMethodArgs, TryFromMethodArgs, UnwrapMethodArgs};
use crate::rpc::{
    CommandSender, Error as RPCError, MethodCall, MethodResult, MethodStatus, PackagedMethod, Properties,
    SessionIdentifier,
};
use crate::spec::basic_types::{List, NamedValue, ObjectReference, TableReference};
use crate::spec::column_types::{ACERef, AuthorityRef, CellBlock, CredentialRef, MethodRef, SPRef};
use crate::spec::{invoking_id::*, method_id::*, table_id};

pub struct SPSession {
    session: SessionIdentifier,
    sender: CommandSender,
}

impl SPSession {
    pub fn new(session: SessionIdentifier, sender: CommandSender, properties: Properties) -> Self {
        sender.open_session(session, properties);
        Self { session, sender }
    }

    async fn do_method_call(&self, call: MethodCall) -> Result<MethodResult, RPCError> {
        let result = self.sender.method(self.session, PackagedMethod::Call(call)).await?;
        match result {
            PackagedMethod::Result(result) => Ok(result),
            _ => {
                let _ = self.sender.method(self.session, PackagedMethod::EndOfSession);
                Err(RPCError::Aborted)
            }
        }
    }
}

impl Drop for SPSession {
    fn drop(&mut self) {
        self.sender.enqueue_method(self.session, PackagedMethod::EndOfSession);
        self.sender.close_session(self.session);
    }
}

impl SPSession {
    pub async fn end_session(self) -> Result<(), RPCError> {
        let result = self.sender.method(self.session, PackagedMethod::EndOfSession).await?;
        self.sender.close_session(self.session); // Make the drop have no effect.
        match result {
            PackagedMethod::EndOfSession => Ok(()),
            _ => Err(RPCError::Aborted),
        }
    }

    pub async fn with<Output: 'static>(mut self, f: impl AsyncFnOnce(&mut SPSession) -> Output) -> Output {
        let result = f(&mut self).await;
        let _ = self.end_session().await;
        result
    }

    pub fn abort_session(self) {
        self.sender.enqueue_method(self.session, PackagedMethod::EndOfSession);
        self.sender.abort_session(self.session);
    }

    pub async fn authenticate(&self, authority: AuthorityRef, proof: Option<&[u8]>) -> Result<bool, RPCError> {
        let call = MethodCall::new_success(THIS_SP, AUTHENTICATE.as_uid(), (authority, proof).into_method_args());
        let results = self.do_method_call(call).await?.take_results()?;
        // I'll assume the result is encoded without the typeOr{} NVP.
        // Not clear in spec, no official examples.
        let (success,) = results.unwrap_method_args().map_err(|_| RPCError::ResultTypeMismatch)?;
        Ok(success)
    }

    pub async fn get<T: TryFrom<Value, Error = Value>>(&self, object: UID, column: u16) -> Result<T, RPCError> {
        Ok(self.get_multiple::<(T,)>(object, column..=column).await?.0)
    }

    pub async fn get_multiple<Tuple: TryFromMethodArgs<Error = MethodStatus>>(
        &self,
        object: UID,
        columns: impl RangeBounds<u16>,
    ) -> Result<Tuple, RPCError> {
        let first_column = match columns.start_bound() {
            std::ops::Bound::Included(n) => *n,
            std::ops::Bound::Excluded(n) => *n + 1,
            core::ops::Bound::Unbounded => 0,
        };

        let call = MethodCall::new_success(object, GET.as_uid(), (CellBlock::object(columns),).into_method_args());
        let results = self.do_method_call(call).await?;
        let results = results.take_results()?;
        // According to the TCG examples, result is encoded without typeOr{} name-value pair.
        let (column_values,): (List<NamedValue<u64, Value>>,) =
            results.unwrap_method_args().map_err(|_| RPCError::ResultTypeMismatch)?;
        let column_values: Vec<_> = column_values
            .0
            .into_iter()
            .map(|nvp| NamedValue { name: nvp.name.wrapping_sub(first_column as u64), ..nvp })
            .collect();
        let mut linearized = Vec::new();
        for column_value in column_values {
            let index = column_value.name as usize;
            let new_size = (index + 1).clamp(linearized.len(), 64);
            linearized.resize(new_size, Value::empty());
            if index < linearized.len() {
                linearized[index] = column_value.value;
            }
        }
        Ok(Tuple::try_from_method_args(linearized).map_err(|_| RPCError::ResultTypeMismatch)?)
    }

    pub async fn set<T: Into<Value>>(&self, object: UID, column: u16, value: T) -> Result<(), RPCError> {
        self.set_multiple(object, [column], (value,)).await
    }

    pub async fn set_multiple<Tuple: IntoMethodArgs, const N: usize>(
        &self,
        object: UID,
        columns: [u16; N],
        values: Tuple,
    ) -> Result<(), RPCError> {
        let where_ = Option::<ObjectReference>::None; // According to the TCG examples, encoded without typeOr{} name-value pair.
        let names = columns;
        let values = values.into_method_args();
        if names.len() != values.len() {
            return Err(MethodStatus::InvalidParameter.into());
        }
        let nvps: Vec<_> = core::iter::zip(names, values).map(|(name, value)| NamedValue { name, value }).collect();
        let nvps = List(nvps);
        let call = MethodCall::new_success(object, SET.as_uid(), (where_, Some(nvps)).into_method_args());
        let _ = self.do_method_call(call).await?.take_results()?; // `Set` returns nothing.
        Ok(())
    }

    pub async fn read(&self, table: TableUID, position: u64, len: u64) -> Result<Vec<u8>, RPCError> {
        let cell_block = CellBlock::bytes(position..(position + len));
        let call = MethodCall::new_success(table.as_uid(), GET.as_uid(), (cell_block,).into_method_args());
        let results = self.do_method_call(call).await?;
        let results = results.take_results()?;
        // According to the TCG examples, result is encoded without typeOr{} name-value pair.
        let (bytes,) = results.unwrap_method_args().map_err(|_| RPCError::ResultTypeMismatch)?;
        Ok(bytes)
    }

    pub async fn write(&self, table: TableUID, position: u64, data: &[u8]) -> Result<(), RPCError> {
        let args = (Some(position), Some(data));
        let call = MethodCall::new_success(table.as_uid(), SET.as_uid(), args.into_method_args());
        let results = self.do_method_call(call).await?;
        let _ = results.take_results()?;
        Ok(())
    }

    pub async fn next(
        &self,
        table: TableReference,
        first: Option<UID>,
        count: Option<u64>,
    ) -> Result<Vec<UID>, RPCError> {
        let call = MethodCall::new_success(table.into(), NEXT.as_uid(), (first, count).into_method_args());
        let results = self.do_method_call(call).await?.take_results()?;
        let (objects,): (List<UID>,) = results.unwrap_method_args().map_err(|_| RPCError::ResultTypeMismatch)?;
        Ok(objects.0)
    }

    pub async fn get_acl(&self, invoking_id: UID, method_id: MethodRef) -> Result<Vec<ACERef>, RPCError> {
        let args = (invoking_id, method_id).into_method_args();
        let call = MethodCall::new_success(table_id::ACCESS_CONTROL.as_uid(), GET_ACL.as_uid(), args);
        let results = self.do_method_call(call).await?.take_results()?;
        let aces: Result<Vec<_>, _> = results.into_iter().map(|value| ACERef::try_from(value)).collect();
        aces.map_err(|_| RPCError::ResultTypeMismatch)
    }

    pub async fn gen_key(
        &self,
        credential_id: CredentialRef,
        public_exponent: Option<u64>,
        pin_length: Option<u16>,
    ) -> Result<(), RPCError> {
        let args = (public_exponent, pin_length).into_method_args();
        let call = MethodCall::new_success(credential_id.into(), GEN_KEY.as_uid(), args);
        let _ = self.do_method_call(call).await?.take_results()?;
        Ok(())
    }

    pub async fn revert(&self, sp: SPRef) -> Result<(), RPCError> {
        let call = MethodCall::new_success(sp.as_uid(), REVERT.as_uid(), vec![]);
        let _ = self.do_method_call(call).await?.take_results()?;
        Ok(())
    }

    pub async fn revert_sp(&self, keep_global_range_key: Option<bool>) -> Result<(), RPCError> {
        let call =
            MethodCall::new_success(THIS_SP.as_uid(), REVERT_SP.as_uid(), (keep_global_range_key,).into_method_args());
        let _ = self.do_method_call(call).await?.take_results()?;
        Ok(())
    }

    pub async fn activate(&self, sp: SPRef) -> Result<(), RPCError> {
        let call = MethodCall::new_success(sp.as_uid(), ACTIVATE.as_uid(), vec![]);
        let _ = self.do_method_call(call).await?.take_results()?;
        Ok(())
    }

    pub async fn random(&self, count: u32, cell: Option<(UID, u16)>) -> Result<Option<Bytes>, RPCError> {
        let cell_block = cell.map(|(object, column)| CellBlock::object_with_table(object, column..=column));
        let call = MethodCall::new_success(THIS_SP, RANDOM.as_uid(), (count, cell_block).into_method_args());
        let results = self.do_method_call(call).await?.take_results()?;
        let (bytes,) = results.unwrap_method_args().map_err(|_| RPCError::ResultTypeMismatch)?;
        Ok(bytes)
    }
}
