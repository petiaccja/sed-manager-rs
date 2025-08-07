//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::sync::atomic::AtomicU32;
use std::collections::HashMap;

use crate::fake_device::data::object_table::{CPINTable, SPTable};
use crate::fake_device::data::security_provider::SecurityProvider;
use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::spec::column_types::{AuthorityRef, BoolOrBytes, LifeCycleState, SPRef};
use crate::spec::opal::admin;
use crate::spec::table_id;

pub struct Controller {
    security_providers: HashMap<SPRef, SecurityProvider>,
    sp_factory: Box<dyn Fn(SPRef) -> SecurityProvider + Send + Sync + 'static>,
    tper_session_number: AtomicU32,
}

impl Controller {
    pub fn new(sp_factory: impl Fn(SPRef) -> SecurityProvider + Send + Sync + 'static, sp_list: &[SPRef]) -> Self {
        Self {
            security_providers: sp_list.iter().map(|sp_ref| (*sp_ref, sp_factory(*sp_ref))).collect(),
            sp_factory: Box::new(sp_factory) as Box<dyn Fn(SPRef) -> SecurityProvider + Send + Sync>,
            tper_session_number: 500.into(),
        }
    }

    pub fn get_security_provider(&self, sp_ref: SPRef) -> Option<&SecurityProvider> {
        self.security_providers.get(&sp_ref)
    }

    pub fn get_security_provider_mut(&mut self, sp_ref: SPRef) -> Option<&mut SecurityProvider> {
        self.security_providers.get_mut(&sp_ref)
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

    pub fn revert_sp(
        &mut self,
        sp_ref: SPRef,
        _keep_global_range_key: Option<bool>,
    ) -> Result<Vec<SPRef>, MethodStatus> {
        if sp_ref == admin::sp::ADMIN {
            for (sp_ref, sp) in self.security_providers.iter_mut() {
                *sp = self.sp_factory.as_ref()(*sp_ref);
            }
            Ok(self.security_providers.keys().cloned().collect())
        } else if let Some(sp) = self.security_providers.get_mut(&sp_ref) {
            *sp = self.sp_factory.as_ref()(sp_ref);
            self.set_life_cycle_state(sp_ref, LifeCycleState::ManufacturedInactive)?;
            Ok(vec![sp_ref])
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }

    pub fn activate(&mut self, sp_ref: SPRef) -> Result<(), MethodStatus> {
        if self.get_life_cycle_state(sp_ref) != Ok(LifeCycleState::ManufacturedInactive) {
            return Err(MethodStatus::InvalidParameter);
        }
        self.set_life_cycle_state(sp_ref, LifeCycleState::Manufactured)?;

        // Copy PINs from Admin SP.
        let sid_c_pin_value = {
            let Some(admin_sp) = self.security_providers.get(&admin::sp::ADMIN) else {
                return Ok(()); // No Admin SP, nothing to copy.
            };
            let Some(admin_c_pins) = admin_sp.get_object_table_specific::<CPINTable>(table_id::C_PIN) else {
                return Ok(());
            };
            let Some(sid_c_pin) = admin_c_pins.get(&admin::c_pin::SID) else {
                return Ok(());
            };
            sid_c_pin.pin.clone()
        };

        let Some(activated_sp) = self.security_providers.get_mut(&sp_ref) else {
            return Err(MethodStatus::InvalidParameter); // No activated SP, that's a problem.
        };
        let Some(activated_c_pins) = activated_sp.get_object_table_specific_mut::<CPINTable>(table_id::C_PIN) else {
            return Ok(());
        };
        for c_pin in activated_c_pins.values_mut() {
            c_pin.pin = sid_c_pin_value.clone();
        }
        Ok(())
    }

    pub fn get_life_cycle_state(&self, sp_ref: SPRef) -> Result<LifeCycleState, MethodStatus> {
        for admin_sp in self.security_providers.values() {
            if let Some(sp_table) = admin_sp.get_object_table_specific::<SPTable>(table_id::SP) {
                if let Some(reverted_sp) = sp_table.get(&sp_ref) {
                    return Ok(reverted_sp.life_cycle_state.clone());
                }
            }
        }
        Err(MethodStatus::InvalidParameter)
    }

    fn set_life_cycle_state(&mut self, sp_ref: SPRef, life_cycle_state: LifeCycleState) -> Result<(), MethodStatus> {
        for admin_sp in self.security_providers.values_mut() {
            if let Some(sp_table) = admin_sp.get_object_table_specific_mut::<SPTable>(table_id::SP) {
                if let Some(reverted_sp) = sp_table.get_mut(&sp_ref) {
                    reverted_sp.life_cycle_state = life_cycle_state;
                    return Ok(());
                }
            }
        }
        Err(MethodStatus::InvalidParameter)
    }
}
