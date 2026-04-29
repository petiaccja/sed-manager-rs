use sed_packet::session_id::SessionId;
use sed_packet::token::{Command, Detokenize, FromTokens, ToTokens};
use sed_packet::{Bytes, Field, FieldRef, MaxBytes, Named, Object, Uid};
use sed_spec::methods::{
    CellBlock, Get, MethodCall, MethodResult, MethodStatus, Random, RandomResult, StartSession, SyncSession,
};
use sed_spec::objects::{AuthorityRef, SecurityProviderRef};
use sed_spec::preconfig::core::shared::invoking_id::{SESSION_MANAGER, THIS_SP};
use sed_spec::preconfig::core::shared::method_id::{GET, RANDOM};
use sed_spec::preconfig::core::shared::sm_method_id::START_SESSION;
use tracing::instrument;

use crate::error::Error;
use crate::protocol::Controller;

/// A session to one of the TPer's security providers.
///
/// Sessions are typically created using [`TPer::start_session`]. This takes
/// care of the communication with the device that establishes the session.
/// If you handle the session setup yourself, you can create independent
/// instances of [`Session`] too.
///
/// The [`Controller`] is shared among all sessions and the `TPer`. This means
/// that the underlying [`Protocol`] will only be shut down once all sessions
/// and the `TPer` are dropped.
///
/// [`TPer::start_session`]: crate::TPer::start_session
/// [`TPer`]: crate::TPer
/// [`Protocol`]: crate::protocol::Protocol
#[derive(Debug)]
pub struct Session {
    session_id: SessionId,
    controller: Controller,
}

impl Session {
    /// Establish a session with the device.
    ///
    /// This method performs the exchange of session startup methods with the
    /// device. Use [`from_started`] if you've already performed the session
    /// startup.
    ///
    /// [`from_started`]: Self::from_started
    #[instrument(level = "info", skip(controller, password), err)]
    pub async fn start(
        controller: Controller,
        host_session_number: u32,
        sp: SecurityProviderRef,
        authority: Option<AuthorityRef>,
        password: Option<MaxBytes<32>>,
    ) -> Result<Self, Error> {
        let call = MethodCall {
            invoking_id: SESSION_MANAGER,
            method_id: START_SESSION,
            parameters: StartSession {
                host_session_id: host_session_number,
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
        let result_tokens = controller
            .call(SessionId::MANAGEMENT, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        let result = MethodCall::<SyncSession>::from_tokens(&result_tokens)?;
        if result.status == MethodStatus::Success {
            let tper_session_id = result.parameters.sp_session_id;
            let session_id = SessionId { hsn: host_session_number, tsn: tper_session_id };
            Ok(Self::from_started(session_id, controller))
        } else {
            Err(result.status.into())
        }
    }

    /// Create a [`Session`] object from an already established session.
    ///
    /// This method assumes that the exchange of session startup methods with
    /// the device has complete successfully and returned the given
    /// `session_id`. In the absence of an already established session, the
    /// device will drop the packets and this session's methods will time out.
    pub fn from_started(session_id: SessionId, controller: Controller) -> Self {
        Self { session_id, controller }
    }

    /// Execute a procedure using this session and then close the session.
    ///
    /// This method ensures that the session has been properly terminated after
    /// it returns. This prevents errors (i.e. "SP busy") during subsequent
    /// communication. These errors could come up if you forget to manually
    /// [`close`] the session, because in that case, the session is dropped
    /// and closed concurrently on another "thread".
    ///
    /// [`close`]: Self::close
    pub async fn with<Output>(self, f: impl AsyncFnOnce(&Self) -> Output) -> Output {
        let result = f(&self).await;
        let _ = self.close().await;
        result
    }

    /// Get one field of an object.
    ///
    /// Note that the TPer may not return the requested field, even if you have
    /// access to them. This is because the SSC specification does not always
    /// require the field to have a value assigned.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn get_field<O, const TABLE: u64, const FIELD: u16>(
        &self,
        field: FieldRef<O, TABLE, FIELD>,
    ) -> Result<<FieldRef<O, TABLE, FIELD> as Field<FIELD>>::Type, Error>
    where
        FieldRef<O, TABLE, FIELD>: Field<FIELD>,
        <FieldRef<O, TABLE, FIELD> as Field<FIELD>>::Type: Detokenize + core::fmt::Debug,
    {
        type FieldType<O, const TABLE: u64, const FIELD: u16> = <FieldRef<O, TABLE, FIELD> as Field<FIELD>>::Type;

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
        // The length of this should be one (or zero) and the only element should be the column requested.
        let mut result = MethodResult::<Vec<Named<u16, FieldType<O, TABLE, FIELD>>>>::from_tokens(&result_tokens)?.0?;
        if let Some(nvp) = result.pop()
            && nvp.name == FIELD
        {
            Ok(nvp.value)
        } else {
            Err(Error::FieldNotReturned)
        }
    }

    /// Get all fields of an object.
    ///
    /// Note that the TPer may not return the all fields, even if you have
    /// access to them. This is because the SSC specification does not always
    /// require the field to have a value assigned.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn get_object<Obj>(&self, object: Obj::Ref) -> Result<Obj, Error>
    where
        Obj: Object + Detokenize + core::fmt::Debug,
        Obj::Ref: Clone + core::fmt::Debug,
        Uid: From<Obj::Ref>,
    {
        let cell_block = CellBlock::object(0..Obj::FIELD_COUNT);

        let call = MethodCall {
            invoking_id: object.into(),
            method_id: GET.to_uid(),
            parameters: Get { cell_block },
            status: MethodStatus::Success,
        };
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        // The length of this should be one (or zero) and the only element should be the column requested.
        Ok(MethodResult::<Obj>::from_tokens(&result_tokens)?.0?)
    }

    /// Generate random bytes.
    ///
    /// This function uses the TPer's built-in random number generation
    /// capabilities. The generated bytes should have a quality suitable for
    /// cryptographic applications.
    ///
    /// # Parameters
    ///
    /// - `count`: the number of random bytes to generate.
    #[instrument(level = "info", skip(self), ret, err)]
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

    /// Explicitly close the session.
    ///
    /// This sends an "end of session" signal to the device and waits for the
    /// response. Once complete, the session is considered closed by both the
    /// TPer and the host.
    ///
    /// This is different from [`Self::drop`], which closes the session in a
    /// separate async task, and does not wait for the result. This may cause
    /// timing issues, and TPer might reply that it's busy when you start
    /// another session.
    #[instrument(level = "info", skip(self), ret, err)]
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

/// Close the session in another task.
///
/// Drop must return immediately, so the session is closed in another async
/// task. Since the current task may continue running concurrently, creating a
/// new session immediately may be refused by the TPer as it's still busy
/// closing the previous one.
impl Drop for Session {
    fn drop(&mut self) {
        let _ = self.controller.call(self.session_id, Command::EndOfSession.to_tokens().expect("invalid token"));
    }
}

#[cfg(test)]
mod tests {
    use sed_spec::objects::{SecurityProvider, SecurityProviderRef, SecurityProviderRefExt as _};

    use super::*;

    #[allow(unused)]
    async fn foo(session: &Session, sp: SecurityProviderRef) -> Result<String, Error> {
        session.get_field(sp.name()).await
    }

    #[allow(unused)]
    async fn bar(session: &Session, sp: SecurityProviderRef) -> Result<SecurityProvider, Error> {
        let result = session.get_object::<SecurityProvider>(sp).await;
        todo!()
    }
}
