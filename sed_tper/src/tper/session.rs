use sed_packet::token::{Command, Detokenize, FromTokens, ToTokens};
use sed_packet::{Bytes, Field, FieldRef, Named, Object};
use sed_spec::methods::{
    CellBlock, Get, GetObjectResult, MethodCall, MethodResult, MethodStatus, Random, RandomResult,
};
use sed_spec::preconfig::core::shared::method_id::GET;
use sed_spec::preconfig::core::shared::{invoking_id::THIS_SP, method_id::RANDOM};
use tracing::instrument;

use crate::error::Error;
use crate::protocol::{Controller, SessionId};

#[derive(Debug)]
pub struct Session {
    session_id: SessionId,
    controller: Controller,
}

impl Session {
    pub fn new(session_id: SessionId, controller: Controller) -> Self {
        Self { session_id, controller }
    }

    pub async fn with<Output>(self, f: impl AsyncFnOnce(&Self) -> Output) -> Output {
        let result = f(&self).await;
        let _ = self.close().await;
        result
    }

    #[instrument(level = "info")]
    pub async fn get_field<O, const TABLE: u64, const FIELD: u16>(
        &self,
        field: FieldRef<O, TABLE, FIELD>,
    ) -> Result<<O as Field<FIELD>>::Type, Error>
    where
        O: Object + Field<FIELD>,
        <O as Field<FIELD>>::Type: Detokenize,
    {
        let call = MethodCall {
            invoking_id: field.object().to_uid(),
            method_id: GET.to_uid(),
            parameters: Get { cell_block: CellBlock::object(field.field()..=field.field()) },
            status: MethodStatus::Success,
        };
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        let result =
            MethodResult::<Named<u16, <O as Field<FIELD>>::Type>>::from_tokens(&result_tokens)?
                .0?;
        if result.name == FIELD {
            Ok(result.value)
        } else {
            Err(Error::ResultTypeMismatch)
        }
    }

    #[instrument(level = "info")]
    pub async fn random(&self, count: usize) -> Result<Bytes, Error> {
        let call = MethodCall {
            invoking_id: THIS_SP,
            method_id: RANDOM.into(),
            parameters: Random { count: count as u64, buffer_out: None },
            status: MethodStatus::Success,
        };
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        let result = MethodResult::<RandomResult>::from_tokens(&result_tokens)?.0?;
        Ok(result.result)
    }

    pub async fn close(self) -> Result<(), Error> {
        // Drop would send another EOS token, which is undesired.
        let this = std::mem::ManuallyDrop::new(self);
        let result_tokens = this
            .controller
            .call(this.session_id, Command::EndOfSession.to_tokens().expect("invalid token"))
            .await
            .map_err(|_| Error::Closed)??;
        let result = Command::from_tokens(&result_tokens)?;
        match result {
            Command::EndOfSession => Ok(()),
            _ => Err(Error::EndOfSessionExpected),
        }
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.controller.call(self.session_id, Command::EndOfSession.to_tokens().expect("invalid token"));
    }
}

#[cfg(test)]
mod tests {
    use sed_spec::objects::{SecurityProvider, SpRef};

    use super::*;

    async fn foo(session: &Session, sp: SpRef) -> Result<String, Error> {
        session.get_field(SecurityProvider::name(sp)).await
    }
}
