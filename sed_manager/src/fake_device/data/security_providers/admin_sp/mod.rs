use as_array::AsArray;

use crate::fake_device::data::access_control_table::AccessControlTable;
use crate::fake_device::data::object_table::{GenericTable, SPTable};
use crate::messaging::uid::{TableUID, UID};
use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::spec::column_types::{ACERef, AuthorityRef, BoolOrBytes, CredentialRef, MethodRef};

use super::basic_sp::BasicSP;
use super::security_provider::SecurityProvider;

mod preconfig_access_control;
mod preconfig_ace;
mod preconfig_authority;
mod preconfig_c_pin;
mod preconfig_sp;
mod preconfig_table;

const ADMIN_IDX: core::ops::RangeInclusive<u64> = 1_u64..=4_u64;

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

pub struct AdminSP {
    pub access_control: AccessControlTable,
    pub basic_sp: BasicSP,
    pub sp_specific: SPSpecific,
}

#[derive(AsArray)]
#[as_array_traits(GenericTable)]
pub struct SPSpecific {
    pub sp: SPTable,
}

impl AdminSP {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecurityProvider for AdminSP {
    fn get_object_table(&self, table: TableUID) -> Option<&dyn GenericTable> {
        let basic = self.basic_sp.as_array().into_iter().find(|table_| table_.uid() == table);
        let specific = self.sp_specific.as_array().into_iter().find(|table_| table_.uid() == table);
        basic.or(specific)
    }

    fn get_object_table_mut(&mut self, table: TableUID) -> Option<&mut dyn GenericTable> {
        let basic = self.basic_sp.as_array_mut().into_iter().find(|table_| table_.uid() == table);
        let specific = self.sp_specific.as_array_mut().into_iter().find(|table_| table_.uid() == table);
        basic.or(specific)
    }

    fn authenticate(&self, authority_id: AuthorityRef, proof: Option<Bytes>) -> Result<BoolOrBytes, MethodStatus> {
        self.basic_sp.authenticate(authority_id, proof)
    }

    fn gen_key(
        &mut self,
        _credential_id: CredentialRef,
        _public_exponent: Option<u64>,
        _pin_length: Option<u16>,
    ) -> Result<(), MethodStatus> {
        Err(MethodStatus::NotAuthorized)
    }

    fn get_acl(&self, invoking_id: UID, method_id: MethodRef) -> Result<Vec<ACERef>, MethodStatus> {
        let entry = self.access_control.get(&invoking_id, &method_id).ok_or(MethodStatus::InvalidParameter)?;
        Ok(entry.acl.0.clone())
    }
}

impl Default for AdminSP {
    fn default() -> Self {
        Self {
            access_control: preconfig_access_control::preconfig_access_control(),
            basic_sp: BasicSP {
                table: preconfig_table::preconfig_table(),
                ace: preconfig_ace::preconfig_ace(),
                authority: preconfig_authority::preconfig_authority(),
                c_pin: preconfig_c_pin::preconfig_c_pin(),
            },
            sp_specific: SPSpecific { sp: preconfig_sp::preconfig_sp() },
        }
    }
}
