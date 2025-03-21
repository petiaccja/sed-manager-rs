//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use core::ops::Deref;

use as_array::AsArray;

use crate::fake_device::data::object_table::{ACETable, AuthorityTable, CPINTable, GenericTable, TableTable};
use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::spec::column_types::{AuthorityRef, BoolOrBytes, CPINRef};

// Admin SP tables:
// --- Basic ---
// - Table
// - SPInfo
// - SPTemplates
// - MethodID
// - AccessControl
// - ACE
// - Authority
// - C_PIN
// --- SP-specific ---
// - TPerInfo
// - Template
// - SP
// - DataRemovalMechanism

#[derive(AsArray)]
#[as_array_traits(GenericTable)]
pub struct BasicSP {
    pub table: TableTable,
    pub ace: ACETable,
    pub authority: AuthorityTable,
    pub c_pin: CPINTable,
}

impl BasicSP {
    pub fn authenticate(&self, authority_id: AuthorityRef, proof: Option<Bytes>) -> Result<BoolOrBytes, MethodStatus> {
        let Some(authority) = self.authority.get(&authority_id) else {
            return Err(MethodStatus::InvalidParameter);
        };
        let credential_id = authority.credential;
        if credential_id.is_null() {
            return Ok(BoolOrBytes::Bool(true));
        };
        if let Ok(c_pin_id) = CPINRef::try_new_other(credential_id) {
            if let Some(credential) = self.c_pin.get(&c_pin_id) {
                let empty_provided_password = vec![];
                let provided_password = proof.as_ref().unwrap_or(&empty_provided_password);
                let success = provided_password == credential.pin.deref().deref();
                Ok(BoolOrBytes::Bool(success))
            } else {
                Err(MethodStatus::TPerMalfunction)
            }
        } else {
            Err(MethodStatus::TPerMalfunction)
        }
    }
}
