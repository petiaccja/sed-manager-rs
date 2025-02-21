use std::ops::Deref;

use as_array::AsArray;

use crate::fake_device::data::objects::{AuthorityTable, CPINTable};
use crate::fake_device::data::table::BasicTable;
use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::spec::column_types::{AuthorityRef, BoolOrBytes, Password};
use crate::spec::table_id;

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
#[as_array_traits(BasicTable)]
pub struct BasicSP {
    pub authorities: AuthorityTable,
    pub c_pin: CPINTable,
}

impl BasicSP {
    pub fn authenticate(&self, authority_id: AuthorityRef, proof: Option<Bytes>) -> Result<BoolOrBytes, MethodStatus> {
        let Some(authority) = self.authorities.get(&authority_id) else {
            return Err(MethodStatus::InvalidParameter);
        };
        let Some(credential_id) = authority.credential else {
            return Ok(BoolOrBytes::Bool(true));
        };
        if credential_id.containing_table() == table_id::C_PIN {
            if let Some(credential) = self.c_pin.get(&credential_id) {
                let empty_provided_password = vec![];
                let empty_authority_password = Password::default();
                let provided_password = proof.as_ref().unwrap_or(&empty_provided_password);
                let authority_password = credential.pin.as_ref().unwrap_or(&empty_authority_password);
                let success = provided_password == authority_password.deref().deref();
                Ok(BoolOrBytes::Bool(success))
            } else {
                Err(MethodStatus::TPerMalfunction)
            }
        } else {
            Err(MethodStatus::TPerMalfunction)
        }
    }
}
