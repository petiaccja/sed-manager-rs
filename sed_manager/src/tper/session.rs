use crate::async_finalize::async_finalize;
use crate::async_finalize::sync_finalize;
use crate::async_finalize::AsyncFinalize;
use crate::messaging::types::AuthorityRef;
use crate::messaging::types::CellBlock;
use crate::messaging::types::List;
use crate::messaging::types::NamedValue;
use crate::messaging::types::ObjectReference;
use crate::messaging::types::TableReference;
use crate::messaging::uid::UID;
use crate::messaging::value::Bytes;
use crate::messaging::value::Value;
use crate::rpc::args::DecodeArgs;
use crate::rpc::args::EncodeArgs;
use crate::rpc::Error as RPCError;
use crate::rpc::MethodCall;
use crate::rpc::MethodResult;
use crate::rpc::MethodStatus;
use crate::rpc::SPSession;
use crate::specification::invoker;
use crate::specification::method;

pub struct Session {
    sp_session: SPSession,
}

impl Session {
    pub fn new(sp_session: SPSession) -> Self {
        Self { sp_session }
    }

    pub async fn end_session(&mut self) -> Result<(), RPCError> {
        let result = self.sp_session.end().await;
        result
    }

    pub async fn authenticate(&self, authority: AuthorityRef, proof: Option<Bytes>) -> Result<bool, RPCError> {
        let call = MethodCall {
            invoking_id: invoker::THIS_SP,
            method_id: method::AUTHENTICATE,
            args: (authority, proof).encode_args(),
            status: MethodStatus::Success,
        };
        let results = get_results(self.sp_session.call(call).await?)?;
        // I'll assume the result is encoded without the typeOr{} NVP.
        // Not clear in spec, no official examples.
        let (success,): (_,) = results.decode_args()?;
        Ok(success)
    }

    pub async fn get<T: TryFrom<Value>>(&self, object: UID, column: u16) -> Result<T, RPCError> {
        let call = MethodCall {
            invoking_id: object,
            method_id: method::GET,
            args: (CellBlock::object(column..=column),).encode_args(),
            status: MethodStatus::Success,
        };
        let results = get_results(self.sp_session.call(call).await?)?;
        // According to the TCG examples, result is encoded without typeOr{} name-value pair.
        let (mut column_values,): (Vec<Value>,) = results.decode_args()?;
        if let Some(value) = column_values.pop() {
            if let Ok(nvp) = NamedValue::<u64, T>::try_from(value) {
                Ok(nvp.value)
            } else {
                Err(RPCError::InvalidColumnType)
            }
        } else {
            Err(MethodStatus::NotAuthorized.into())
        }
    }

    pub async fn set<T: Into<Value>>(&self, object: UID, column: u16, value: T) -> Result<(), RPCError> {
        let where_ = Option::<ObjectReference>::None; // According to the TCG examples, encoded without typeOr{} name-value pair.
        let values = Some(vec![Value::from(NamedValue { name: column, value })]); // According to the TCG examples, encoded without typeOr{} name-value pair.
        let call = MethodCall {
            invoking_id: object,
            method_id: method::SET,
            args: (where_, values).encode_args(),
            status: MethodStatus::Success,
        };
        let _ = get_results(self.sp_session.call(call).await?)?; // The Set method returns nothing.
        Ok(())
    }

    pub async fn next(
        &self,
        table: TableReference,
        first: Option<ObjectReference>,
        count: Option<u64>,
    ) -> Result<List<ObjectReference>, RPCError> {
        let call = MethodCall {
            invoking_id: table.into(),
            method_id: method::NEXT,
            args: (first, count).encode_args(),
            status: MethodStatus::Success,
        };
        let results = get_results(self.sp_session.call(call).await?)?;
        let (objects,): (_,) = results.decode_args()?;
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

impl AsyncFinalize for Session {
    async fn finalize(&mut self) {
        let _ = self.end_session().await;
        async_finalize(&mut self.sp_session).await;
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        sync_finalize(self);
    }
}

fn get_results(method_result: MethodResult) -> Result<Vec<Value>, RPCError> {
    if method_result.status == MethodStatus::Success {
        Ok(method_result.results)
    } else {
        Err(method_result.status.into())
    }
}
