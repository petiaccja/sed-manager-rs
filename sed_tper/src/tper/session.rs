use std::ops::RangeBounds;
use std::sync::atomic::{AtomicBool, Ordering};

use sed_packet::session_id::SessionId;
use sed_packet::token::{Command, Detokenize, Detokenizer, FromTokens, ToTokens, Tokenize};
use sed_packet::{Bytes, Field, FieldRef, Ignore, MaxBytes, Named, Object, ObjectRef, TableRef, Uid};
use sed_spec::methods::{
    Activate, Authenticate, AuthenticateResult, CellBlock, GenKey, Get, GetAcl, GetBytesResult, MethodCall,
    MethodResult, MethodStatus, Next, Random, Revert, RevertSp, SessionMethodParam as _, SetBytes, SetObject,
    SetResult, StartSession, SyncSession,
};
use sed_spec::objects::{AceRef, AuthorityRef, CredentialRef, MethodRef, SecurityProviderRef};
use sed_spec::preconfig::core::shared::invoking_id::{SESSION_MANAGER, THIS_SP};
use sed_spec::preconfig::core::shared::sm_method_id::START_SESSION;
use sed_spec::preconfig::core::shared::{method_id, table_id};
use sed_spec_macros::DetokenizeStruct;
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
    security_provider: SecurityProviderRef,
    session_id: SessionId,
    controller: Controller,
    needs_eos: AtomicBool,
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
            Ok(Self::from_started(sp, session_id, controller))
        } else {
            Err(result.status.into())
        }
    }

    /// Create a [`Session`] object from an already established session.
    ///
    /// This method assumes that the exchange of session startup methods with
    /// the device has complete successfully and returned the given
    /// `session_id`.
    ///
    /// Additionally, the protocol stack must also have been informed about the
    /// startup via the controller.
    ///
    /// If the preconditions are not met, requests will be dropped or will time
    /// out.
    pub fn from_started(security_provider: SecurityProviderRef, session_id: SessionId, controller: Controller) -> Self {
        Self { security_provider, session_id, controller, needs_eos: true.into() }
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

    /// Activate the security provider
    ///
    /// This method is typically called on the Locking SP of the TPer. When
    /// successful, the security provider transitions from the
    /// [`ManufacturedInactive`] state to the [`Manufactured`] state and its
    /// functionality becomes available.
    ///
    /// To start a session on the SP as Admin1, you can use the SID authority's
    /// password at the time of the activation.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn activate(&self, sp: SecurityProviderRef) -> Result<(), Error> {
        let call = Activate.to_call(sp.to_uid());
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        let _result = Activate::result_from_tokens(&result_tokens)??;
        Ok(())
    }

    /// Authenticate as an authority, with a password when required.
    ///
    /// After a successful authentication, you can perform actions for which the
    /// authority has permission. You may authenticate to multiple authorities,
    /// at any time, but the device may impose limitations on how many
    /// authorities can be authenticated at a time.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn authenticate(&self, authority: AuthorityRef, password: Option<MaxBytes<32>>) -> Result<(), Error> {
        let params = Authenticate { authority, proof: password };
        let call = params.to_call(THIS_SP);
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        let result = Authenticate::result_from_tokens(&result_tokens)??;
        match result {
            AuthenticateResult::Success(true) => Ok(()),
            AuthenticateResult::Success(false) => Err(MethodStatus::NotAuthorized.into()),
            AuthenticateResult::Challenge(_) => Err(Error::NotImplemented),
        }
    }

    /// Generate a random credential for one of the credential objects.
    ///
    /// Credential objects (marked by [`CredentialRef`]) have a credential or
    /// key as one of their fields, and you can use this function to generate
    /// a random value for that field.
    ///
    /// This function is most commonly used to generate a new encryption key
    /// in the `K_AES_*` tables for one of the locking ranges. This will erase
    /// (i.e. crypto-shred) the data in that locking range.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn gen_key(
        &self,
        credential: impl CredentialRef,
        public_exponent: Option<u64>,
        pin_length: Option<u8>,
    ) -> Result<(), Error> {
        let call = GenKey { public_exponent, pin_length }.to_call(credential.into());
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        let _result = GenKey::result_from_tokens(&result_tokens)??;
        Ok(())
    }

    /// Get the access control list (ACL) for the `invoking_id` / `method_id` pair.
    ///
    /// The ACL is a list of references into the ACE table, that is, a list of
    /// access control elements (ACEs). Authorities that are mentioned in the
    /// ACEs have permissions perform `method_id` on the `invoking_id`.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn get_acl(&self, invoking_id: Uid, method_id: MethodRef) -> Result<Vec<AceRef>, Error> {
        let call = GetAcl { invoking_id, method_id }.to_call(table_id::ACCESS_CONTROL.into());
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        let result = GetAcl::result_from_tokens(&result_tokens)??;
        Ok(result.acl)
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

        let parameters = Get { cell_block: CellBlock::object(field.field()..=field.field()) };
        let call = parameters.to_call(field.object().to_uid());
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        let result =
            MethodResult::<SingleFieldParamList<FieldType<O, TABLE, FIELD>, FIELD>>::from_tokens(&result_tokens)?.0?;
        match result.field {
            Some(value) => Ok(value),
            None => Err(Error::FieldNotReturned),
        }
    }

    /// Get all fields of an object.
    ///
    /// Note that the TPer may not return all fields, even if you have
    /// access to them. This is because the SSC specification does not always
    /// require the field to have a value assigned.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn get_object<Obj>(
        &self,
        object: Obj::Ref,
        fields: impl RangeBounds<u16> + core::fmt::Debug,
    ) -> Result<Obj, Error>
    where
        Obj: Object + Detokenize + core::fmt::Debug,
        Obj::Ref: Clone + core::fmt::Debug,
        Uid: From<Obj::Ref>,
    {
        let cell_block = CellBlock::object(fields);
        let parameters = Get { cell_block };
        let call = parameters.to_call(object.into());
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        Ok(MethodResult::<WholeObjectParamList<Obj>>::from_tokens(&result_tokens)?.0?.object)
    }

    /// Read a slice of a byte table.
    ///
    /// # Parameters
    /// - `table`: must be a reference to a byte table, object tables will fail.
    /// - `bytes`: the range of bytes to read. Unbounded ranges will read from
    ///   the start of the table or until the end of the table.
    #[instrument(level = "info", skip(self), err)]
    pub async fn get_bytes(
        &self,
        table: TableRef,
        bytes: impl RangeBounds<u64> + core::fmt::Debug,
    ) -> Result<Vec<u8>, Error> {
        let cell_block = CellBlock::bytes_with_table(table, bytes);
        let parameters = Get { cell_block };
        let call = parameters.to_call(table.into());
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        Ok(MethodResult::<GetBytesResult>::from_tokens(&result_tokens)?.0?.result.0)
    }

    /// Iterate over objects in a table.
    ///
    /// This method returns the next `count` objects in the table, starting with
    /// the object immediately after `where_`. If `where_` isn't provided, it
    /// starts with the first object in the table, and if `count` isn't provided,
    /// object are returned all the way to the last.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn next<const TABLE: u64>(
        &self,
        where_: Option<ObjectRef<TABLE>>,
        count: Option<u64>,
    ) -> Result<Vec<ObjectRef<TABLE>>, Error> {
        // This should be const, but generic parameters are not accessible in const :(.
        let table: TableRef = TableRef::new(TABLE);
        let parameters = Next { where_, count };
        let call = parameters.to_call(table.into());
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        Ok(Next::result_from_tokens(&result_tokens)??.result)
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
    pub async fn random(&self, count: usize) -> Result<Vec<u8>, Error> {
        let parameters = Random { count: count as u64, buffer_out: None };
        let call = parameters.to_call(THIS_SP);
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        let result = Random::result_from_tokens(&result_tokens)??;
        Ok(result.result.0)
    }

    /// Revert the security provider to its factory original state.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn revert(&self, sp: SecurityProviderRef) -> Result<(), Error> {
        let parameters = Revert {};
        let call = parameters.to_call(sp.into());
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        let result = Revert::result_from_tokens(&result_tokens)?;
        match result {
            Ok(_) => {
                // If we revert the SP on which this session is running, this (and all
                // other sessions on the SP) will be terminated. After that, all methods,
                // including `close`, will time out, so let's not send and EOS.
                // Note that `revert` can only be called from an Admin SP session, so
                // a successful revert on the secondary SP will never terminate this session.
                if sp == self.security_provider {
                    self.needs_eos.store(false, Ordering::Relaxed);
                }
                Ok(())
            }
            Err(err) => Err(err.into()),
        }
    }

    /// Revert the security provider to its factory original state.
    ///
    /// After reverting the SP, the session is immediately aborted.
    pub async fn revert_sp(self, keep_global_range_key: Option<bool>) -> Result<(), (Self, Error)> {
        #[instrument(level = "info", skip(self_), ret, err)]
        async fn do_revert_sp(self_: &Session, keep_global_range_key: Option<bool>) -> Result<(), Error> {
            let parameters = RevertSp { keep_global_range_key };
            let call = parameters.to_call(THIS_SP);
            let result_tokens = self_
                .controller
                .call(self_.session_id, call.to_tokens().expect("invalid method call"))
                .await
                .map_err(|_| Error::Closed)??;
            let _result = RevertSp::result_from_tokens(&result_tokens)??;
            Ok(())
        }

        match do_revert_sp(&self, keep_global_range_key).await {
            Ok(_) => {
                // The revert function succeeded, meaning the session is aborted. No need
                // to send an EOS on drop.
                self.needs_eos.store(false, Ordering::Relaxed);
                Ok(())
            }
            Err(err) => {
                // The revert function failed, the session is likely alive, but
                // we cannot tell in case of an I/O error happened during
                // IF-RECV. Let's assume the session is alive.
                Err((self, err))
            }
        }
    }

    /// Set a field of an object.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn set_field<O, const TABLE: u64, const FIELD: u16>(
        &self,
        field: FieldRef<O, TABLE, FIELD>,
        value: <FieldRef<O, TABLE, FIELD> as Field<FIELD>>::Type,
    ) -> Result<(), Error>
    where
        FieldRef<O, TABLE, FIELD>: Field<FIELD>,
        <FieldRef<O, TABLE, FIELD> as Field<FIELD>>::Type: Tokenize + core::fmt::Debug,
    {
        let parameters = vec![Named { name: 1u16, value: vec![Named { name: FIELD, value: value }] }];
        let call = MethodCall {
            invoking_id: field.object().into(),
            method_id: method_id::SET.into(),
            parameters,
            status: MethodStatus::Success,
        };
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        let _result = MethodResult::<SetResult>::from_tokens(&result_tokens)?.0?;
        Ok(())
    }

    /// Set multiple fields of an object.
    ///
    /// Only the fields that are non-`None` in `value` are updated in the TPer.
    #[instrument(level = "info", skip(self), ret, err)]
    pub async fn set_object<Obj>(&self, object: Obj::Ref, value: Obj) -> Result<(), Error>
    where
        Obj: Object + Tokenize + core::fmt::Debug,
        Obj::Ref: Tokenize + Into<Uid> + core::fmt::Debug,
    {
        let parameters = SetObject { where_: None, values: Some(value) };
        let call = parameters.to_call(object.into());
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        let _result = SetObject::<Obj>::result_from_tokens(&result_tokens)??;
        Ok(())
    }

    /// Write a some bytes to a table.
    ///
    /// This function overwrites the bytes at the given location. The table's
    /// size is not changed.
    ///
    /// # Parameters
    /// - `table`: must be a reference to a byte table, object tables will fail.
    /// - `where_`: the byte offset to where the `bytes` should be written.
    /// - `bytes`: the bytes to write into the table.
    #[instrument(level = "info", skip(self, bytes), fields(bytes_len=bytes.len()), err)]
    pub async fn set_bytes(&self, table: TableRef, where_: u64, bytes: &[u8]) -> Result<(), Error> {
        let parameters = SetBytes { where_: Some(where_), values: Some(Bytes(bytes.into())) };
        let call = parameters.to_call(table.into());
        let result_tokens = self
            .controller
            .call(self.session_id, call.to_tokens().expect("invalid method call"))
            .await
            .map_err(|_| Error::Closed)??;
        let _result = SetBytes::result_from_tokens(&result_tokens)??;
        Ok(())
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
        let needs_eos = self.needs_eos.swap(false, Ordering::Relaxed);
        if needs_eos {
            let result_tokens = self
                .controller
                .call(self.session_id, Command::EndOfSession.to_tokens().expect("invalid token"))
                .await
                .map_err(|_| Error::Closed)??;
            let result = Command::from_tokens(&result_tokens)?;
            match result {
                Command::EndOfSession => Ok(()),
                _ => Err(Error::EndOfSessionExpected),
            }
        } else {
            Ok(())
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
        let needs_eos = self.needs_eos.swap(false, Ordering::Relaxed);
        if needs_eos {
            let _ = self.controller.call(self.session_id, Command::EndOfSession.to_tokens().expect("invalid token"));
        }
    }
}

struct SingleFieldParamList<T, const INDEX: u16> {
    field: Option<T>,
}

impl<T, const INDEX: u16> Detokenize for SingleFieldParamList<T, INDEX>
where
    T: Detokenize,
{
    fn detokenize<D: Detokenizer>(detokenizer: &mut D) -> Result<Self, D::Error> {
        let mut field = None;
        detokenizer.detokenize_list(|de| {
            de.detokenize_list(|de| {
                de.detokenize_named(
                    |de| u16::detokenize(de),
                    |de, name| {
                        match name {
                            x if x == &INDEX => field = Some(T::detokenize(de)?),
                            _ => drop(Ignore::detokenize(de)?),
                        };
                        Ok(())
                    },
                )
                .map(|_| ())
            })
        })?;
        Ok(Self { field })
    }
}

#[derive(DetokenizeStruct)]
struct WholeObjectParamList<O: Detokenize> {
    object: O,
}
