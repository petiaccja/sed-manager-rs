//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::HashMap;

use crate::fake_device::data::object_table::SPTable;
use crate::fake_device::data::security_provider::SecurityProvider;
use crate::rpc::MethodStatus;
use crate::spec::column_types::{LifeCycleState, SPRef};
use crate::spec::{opal, table_id};

pub struct TPer {
    pub security_providers: HashMap<SPRef, SecurityProvider>,
    pub sp_factory: Box<dyn Fn(SPRef) -> SecurityProvider + Send + Sync + 'static>,
}

impl TPer {
    pub fn new(sp_factory: impl Fn(SPRef) -> SecurityProvider + Send + Sync + 'static, sp_list: &[SPRef]) -> Self {
        Self {
            security_providers: sp_list.iter().map(|sp_ref| (*sp_ref, sp_factory(*sp_ref))).collect(),
            sp_factory: Box::new(sp_factory) as Box<dyn Fn(SPRef) -> SecurityProvider + Send + Sync>,
        }
    }

    pub fn get_sp(&self, sp_ref: SPRef) -> Option<&SecurityProvider> {
        self.security_providers.get(&sp_ref)
    }

    pub fn get_sp_mut(&mut self, sp_ref: SPRef) -> Option<&mut SecurityProvider> {
        self.security_providers.get_mut(&sp_ref)
    }

    pub fn get_admin_sp(&self) -> Option<&SecurityProvider> {
        self.get_admin_sp_uid().map(|sp_uid| self.security_providers.get(&sp_uid)).flatten()
    }

    pub fn get_admin_sp_mut(&mut self) -> Option<&mut SecurityProvider> {
        self.get_admin_sp_uid().map(|sp_uid| self.security_providers.get_mut(&sp_uid)).flatten()
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

    pub fn set_life_cycle_state(&mut self, sp_ref: SPRef, life_cycle_state: LifeCycleState) -> Result<(), MethodStatus> {
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
