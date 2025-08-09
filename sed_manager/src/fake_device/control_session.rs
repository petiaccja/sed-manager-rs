//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::messaging::value::Bytes;
use crate::rpc::{MethodStatus, Properties, SessionIdentifier};
use crate::spec::basic_types::{List, NamedValue};
use crate::spec::column_types::{AuthorityRef, MaxBytes32, SPRef};

use super::data::TPer;
use super::sp_session::SPSession;

pub struct ControlSession {
    capabilities: Properties,
    properties: Properties,
}

pub struct ControlSessionExecutor<'session, 'tper> {
    session: &'session mut ControlSession,
    tper: &'tper mut TPer,
}

impl ControlSession {
    pub fn new(capabilities: Properties) -> Self {
        Self { capabilities, properties: Properties::ASSUMED }
    }

    pub fn on_tper<'me, 'tper>(&'me mut self, tper: &'tper mut TPer) -> ControlSessionExecutor<'me, 'tper> {
        ControlSessionExecutor { session: self, tper: tper }
    }
}

impl<'session, 'tper> ControlSessionExecutor<'session, 'tper> {
    pub fn properties(
        &mut self,
        host_properties: Option<List<NamedValue<MaxBytes32, u32>>>,
    ) -> Result<(List<NamedValue<MaxBytes32, u32>>, Option<List<NamedValue<MaxBytes32, u32>>>), MethodStatus> {
        let host_properties = host_properties.unwrap_or(List::new());
        let host_properties = Properties::from_list(host_properties.as_slice());
        let common_properties = Properties::common(&self.session.capabilities, &host_properties);
        let capabilities = self.session.capabilities.to_list();
        let common_properties = common_properties.to_list();
        Ok((capabilities, Some(common_properties)))
    }

    pub fn start_session(
        &mut self,
        hsn: u32,
        sp_uid: SPRef,
        write: bool,
        host_challenge: Option<Bytes>,
        host_exch_auth: Option<AuthorityRef>,
        host_exch_cert: Option<Bytes>,
        host_sgn_auth: Option<AuthorityRef>,
        host_sgn_cert: Option<Bytes>,
        session_timeout: Option<u32>,
        trans_timeout: Option<u32>,
        initial_credit: Option<u32>,
        signed_hash: Option<Bytes>,
    ) -> Result<
        (u32, u32, Option<Bytes>, Option<Bytes>, Option<Bytes>, Option<u32>, Option<u32>, Option<Bytes>),
        MethodStatus,
    > {
        let result = {
            self.tper.start_session(
                hsn,
                sp_uid,
                write,
                host_challenge,
                host_exch_auth,
                host_exch_cert,
                host_sgn_auth,
                host_sgn_cert,
                session_timeout,
                trans_timeout,
                initial_credit,
                signed_hash,
            )
        };
        match result {
            Ok(sync_session) => {
                let id = SessionIdentifier { hsn: sync_session.0, tsn: sync_session.1 };
                let sp_session = SPSession::new(sp_uid, write, host_sgn_auth, self.tper.clone());
                self.security_provider_sessions.insert(id, sp_session);
                Ok(sync_session)
            }
            Err(err) => Err(err),
        }
    }
}
