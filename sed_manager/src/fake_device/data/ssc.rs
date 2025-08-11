//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::HashMap;

use crate::fake_device::data::object_table::{CPINTable, SPTable};
use crate::fake_device::data::security_provider::SecurityProvider;
use crate::rpc::MethodStatus;
use crate::spec::column_types::{LifeCycleState, SPRef};
use crate::spec::{self, opal, table_id};

pub struct SecuritySubsystemClass {
    pub security_providers: HashMap<SPRef, SecurityProvider>,
    pub make_factory_sp: Box<dyn Fn(SPRef) -> SecurityProvider + Send + Sync + 'static>,
}

impl SecuritySubsystemClass {
    pub fn new(
        make_factory_sp: impl Fn(SPRef) -> SecurityProvider + Send + Sync + 'static,
        security_providers: &[SPRef],
    ) -> Self {
        Self {
            security_providers: security_providers.iter().map(|sp| (*sp, make_factory_sp(*sp))).collect(),
            make_factory_sp: Box::new(make_factory_sp) as Box<dyn Fn(SPRef) -> SecurityProvider + Send + Sync>,
        }
    }

    pub fn get_sp(&self, sp_ref: SPRef) -> Option<&SecurityProvider> {
        self.security_providers.get(&sp_ref)
    }

    pub fn get_sp_mut(&mut self, sp_ref: SPRef) -> Option<&mut SecurityProvider> {
        self.security_providers.get_mut(&sp_ref)
    }

    pub fn list_sps(&self) -> impl Iterator<Item = &SPRef> {
        self.security_providers.keys()
    }

    pub fn get_admin_sp(&self) -> Option<&SecurityProvider> {
        self.get_admin_sp_uid().map(|sp_uid| self.security_providers.get(&sp_uid)).flatten()
    }

    pub fn get_admin_sp_mut(&mut self) -> Option<&mut SecurityProvider> {
        self.get_admin_sp_uid().map(|sp_uid| self.security_providers.get_mut(&sp_uid)).flatten()
    }

    pub fn activate_sp(&mut self, sp_ref: SPRef) -> Result<(), MethodStatus> {
        if self.get_life_cycle_state(sp_ref)? != LifeCycleState::ManufacturedInactive {
            return Err(MethodStatus::InvalidParameter);
        }
        self.set_life_cycle_state(sp_ref, LifeCycleState::Manufactured)?;

        // Copy PINs from Admin SP.
        let sid_c_pin_value = {
            let Some(admin_sp) = self.get_admin_sp() else {
                return Ok(()); // No Admin SP, nothing to copy.
            };
            let Some(admin_c_pins) = admin_sp.get_object_table_specific::<CPINTable>(table_id::C_PIN) else {
                return Ok(());
            };
            // We'll hardcode for Opal, but all SSCs have the same SID.
            let Some(sid_c_pin) = admin_c_pins.get(&spec::opal::admin::c_pin::SID) else {
                return Ok(());
            };
            sid_c_pin.pin.clone()
        };

        let activated_sp = self.get_sp_mut(sp_ref).ok_or(MethodStatus::InvalidParameter)?;
        let Some(activated_c_pins) = activated_sp.get_object_table_specific_mut::<CPINTable>(table_id::C_PIN) else {
            return Ok(());
        };
        for c_pin in activated_c_pins.values_mut() {
            c_pin.pin = sid_c_pin_value.clone();
        }
        Ok(())
    }

    pub fn revert_sp(&mut self, sp_ref: SPRef) -> Result<Vec<SPRef>, MethodStatus> {
        let admin_sp_ref = self.get_admin_sp_uid().ok_or(MethodStatus::TPerMalfunction)?;
        if sp_ref == admin_sp_ref {
            // Revert all security providers.
            for (sp_ref, sp) in &mut self.security_providers {
                *sp = (self.make_factory_sp)(*sp_ref);
            }
            Ok(self.security_providers.keys().cloned().collect())
        } else {
            // Revert only the specified security provider.
            self.set_life_cycle_state(sp_ref, LifeCycleState::ManufacturedInactive)?;
            let new_sp = (self.make_factory_sp)(sp_ref);
            let sp = self.get_sp_mut(sp_ref).ok_or(MethodStatus::InvalidParameter)?;
            *sp = new_sp;
            Ok(vec![sp_ref])
        }
    }

    pub fn get_life_cycle_state(&self, sp_ref: SPRef) -> Result<LifeCycleState, MethodStatus> {
        let admin_sp = self.get_admin_sp().ok_or(MethodStatus::TPerMalfunction)?;
        if let Some(sp_table) = admin_sp.get_object_table_specific::<SPTable>(table_id::SP) {
            if let Some(sp_obj) = sp_table.get(&sp_ref) {
                return Ok(sp_obj.life_cycle_state);
            }
        }
        Err(MethodStatus::InvalidParameter)
    }

    pub fn set_life_cycle_state(
        &mut self,
        sp_ref: SPRef,
        life_cycle_state: LifeCycleState,
    ) -> Result<(), MethodStatus> {
        let admin_sp = self.get_admin_sp_mut().ok_or(MethodStatus::TPerMalfunction)?;
        if let Some(sp_table) = admin_sp.get_object_table_specific_mut::<SPTable>(table_id::SP) {
            if let Some(sp_obj) = sp_table.get_mut(&sp_ref) {
                sp_obj.life_cycle_state = life_cycle_state;
                return Ok(());
            }
        }
        Err(MethodStatus::InvalidParameter)
    }

    pub fn get_admin_sp_uid(&self) -> Option<SPRef> {
        // Most devices will have the Opal the Admin SP under the Opal Admin SP's UID.
        let default_admin_sp = opal::admin::sp::ADMIN;
        if self.security_providers.contains_key(&default_admin_sp) {
            Some(default_admin_sp)
        } else {
            // The Admin SP should have the SP table.
            for (sp_uid, sp) in &self.security_providers {
                if sp.get_object_table_specific::<SPTable>(table_id::SP).is_some() {
                    return Some(*sp_uid);
                }
            }
            None
        }
    }
}
