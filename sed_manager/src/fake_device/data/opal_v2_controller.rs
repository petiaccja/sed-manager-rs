use core::sync::atomic::AtomicU32;

use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::spec::column_types::{AuthorityRef, BoolOrBytes, LifeCycleState, SPRef};
use crate::spec::opal::{admin, locking};

use super::security_providers::{AdminSP, LockingSP, SecurityProvider};

#[derive(Default)]
pub struct OpalV2Controller {
    pub admin_sp: AdminSP,
    pub locking_sp: LockingSP,
    tper_session_number: AtomicU32,
}

impl OpalV2Controller {
    pub fn new() -> Self {
        Self { admin_sp: AdminSP::default(), locking_sp: LockingSP::default(), tper_session_number: 500.into() }
    }

    pub fn get_security_provider(&self, sp: SPRef) -> Option<&dyn SecurityProvider> {
        match sp {
            admin::sp::ADMIN => Some(&self.admin_sp),
            admin::sp::LOCKING => Some(&self.locking_sp),
            _ => None,
        }
    }

    pub fn get_security_provider_mut(&mut self, sp: SPRef) -> Option<&mut dyn SecurityProvider> {
        match sp {
            admin::sp::ADMIN => Some(&mut self.admin_sp),
            admin::sp::LOCKING => Some(&mut self.locking_sp),
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
            Some(auth) => security_provider.authenticate(auth, host_challenge)? == BoolOrBytes::Bool(true),
            None => true,
        };
        if !authenticated {
            return Err(MethodStatus::NotAuthorized);
        }

        let tsn = self.tper_session_number.fetch_add(1, core::sync::atomic::Ordering::Relaxed);
        Ok((hsn, tsn, None, None, None, None, None, None))
    }

    pub fn revert(&mut self, sp: SPRef) -> Result<Vec<SPRef>, MethodStatus> {
        self.revert_sp(sp, None)
    }

    pub fn revert_sp(&mut self, sp: SPRef, _keep_global_range_key: Option<bool>) -> Result<Vec<SPRef>, MethodStatus> {
        match sp {
            admin::sp::ADMIN => {
                self.admin_sp = AdminSP::new();
                self.locking_sp = LockingSP::new();
                Ok(vec![admin::sp::ADMIN, admin::sp::LOCKING])
            }
            admin::sp::LOCKING => {
                let row_sp_locking = self.admin_sp.sp_specific.sp.get_mut(&sp).unwrap();
                row_sp_locking.life_cycle_state = LifeCycleState::ManufacturedInactive;

                self.locking_sp = LockingSP::new();
                Ok(vec![admin::sp::LOCKING])
            }
            _ => Err(MethodStatus::InvalidParameter),
        }
    }

    pub fn activate(&mut self, sp: SPRef) -> Result<(), MethodStatus> {
        match sp {
            admin::sp::LOCKING => {
                let row_sp_locking = self.admin_sp.sp_specific.sp.get_mut(&sp).unwrap();
                row_sp_locking.life_cycle_state = LifeCycleState::Manufactured;

                let row_c_pin_sid = self.admin_sp.basic_sp.c_pin.get(&admin::c_pin::SID).unwrap();
                let row_c_pin_admin1 =
                    self.locking_sp.basic_sp.c_pin.get_mut(&locking::c_pin::ADMIN.nth(1).unwrap()).unwrap();
                row_c_pin_admin1.pin = row_c_pin_sid.pin.clone();
                Ok(())
            }
            _ => Err(MethodStatus::InvalidParameter),
        }
    }
}
