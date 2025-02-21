use std::sync::atomic::AtomicU32;

use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::spec::column_types::{AuthorityRef, BoolOrBytes, SPRef};
use crate::spec::opal::admin::sp;

use super::security_providers::{AdminSP, LockingSP, SecurityProvider};

#[derive(Default)]
pub struct OpalV2Controller {
    admin_sp: AdminSP,
    locking_sp: LockingSP,
    tper_session_number: AtomicU32,
}

impl OpalV2Controller {
    pub fn new() -> Self {
        Self { admin_sp: AdminSP::default(), locking_sp: LockingSP::default(), tper_session_number: 1.into() }
    }

    pub fn get_security_provider(&self, sp: SPRef) -> Option<&dyn SecurityProvider> {
        match sp {
            sp::ADMIN => Some(&self.admin_sp),
            sp::LOCKING => Some(&self.locking_sp),
            _ => None,
        }
    }

    pub fn get_security_provider_mut(&mut self, sp: SPRef) -> Option<&mut dyn SecurityProvider> {
        match sp {
            sp::ADMIN => Some(&mut self.admin_sp),
            sp::LOCKING => Some(&mut self.locking_sp),
            _ => None,
        }
    }

    pub fn start_session(
        &self,
        hsn: u32,
        sp_uid: SPRef,
        _write: bool,
        host_challenge: Option<Bytes>,
        _host_exch_auth: Option<AuthorityRef>,
        _host_exch_cert: Option<Bytes>,
        host_sgn_auth: Option<AuthorityRef>,
        _host_sgn_cert: Option<Bytes>,
        _session_timeout: Option<u32>,
        _trans_timeout: Option<u32>,
        _initial_credit: Option<u32>,
        _signed_hash: Option<Bytes>,
    ) -> Result<
        (u32, u32, Option<Bytes>, Option<Bytes>, Option<Bytes>, Option<u32>, Option<u32>, Option<Bytes>),
        MethodStatus,
    > {
        let Some(security_provider) = self.get_security_provider(sp_uid) else {
            return Err(MethodStatus::InvalidParameter);
        };

        let authenticated = match host_sgn_auth {
            Some(auth) => security_provider.authenticate(auth, host_challenge) == Ok(BoolOrBytes::Bool(true)),
            None => true,
        };
        if !authenticated {
            return Err(MethodStatus::NotAuthorized);
        }

        let tsn = self.tper_session_number.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        Ok((hsn, tsn, None, None, None, None, None, None))
    }
}
