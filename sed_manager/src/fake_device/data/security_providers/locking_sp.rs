use as_array::AsArray;

use crate::fake_device::data::objects::{AuthorityTable, CPINTable};
use crate::fake_device::data::table::BasicTable;
use crate::messaging::uid::TableUID;
use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::spec::column_types::{AuthorityRef, BoolOrBytes};

use super::basic_sp::BasicSP;
use super::security_provider::SecurityProvider;

// Locking SP tables:
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
// - SecretProtect
// - LockingInfo
// - Locking
// - MBRControl
// - MBR
// - K_AES_128
// - K_AES_256
// - DataStore

pub struct LockingSP {
    pub basic_sp: BasicSP,
    sp_specific: SPSpecific,
}

#[derive(AsArray)]
#[as_array_traits(BasicTable)]
struct SPSpecific {}

impl LockingSP {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecurityProvider for LockingSP {
    fn get_table(&self, table: TableUID) -> Option<&dyn BasicTable> {
        let basic = self.basic_sp.as_array().into_iter().find(|table_| table_.uid() == table);
        let specific = self.sp_specific.as_array().into_iter().find(|table_| table_.uid() == table);
        basic.or(specific)
    }

    fn get_table_mut(&mut self, table: TableUID) -> Option<&mut dyn BasicTable> {
        let basic = self.basic_sp.as_array_mut().into_iter().find(|table_| table_.uid() == table);
        let specific = self.sp_specific.as_array_mut().into_iter().find(|table_| table_.uid() == table);
        basic.or(specific)
    }

    fn authenticate(&self, authority_id: AuthorityRef, proof: Option<Bytes>) -> Result<BoolOrBytes, MethodStatus> {
        self.basic_sp.authenticate(authority_id, proof)
    }
}

impl Default for LockingSP {
    fn default() -> Self {
        let authorities = AuthorityTable::new();
        let c_pin = CPINTable::new();
        Self { basic_sp: BasicSP { authorities, c_pin }, sp_specific: SPSpecific {} }
    }
}
