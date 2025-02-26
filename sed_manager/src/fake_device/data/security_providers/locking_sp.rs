use as_array::AsArray;

use crate::fake_device::data::objects::{Authority, AuthorityTable, CPINTable, LockingRange, LockingTable, CPIN};
use crate::fake_device::data::table::GenericTable;
use crate::messaging::uid::TableUID;
use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::spec::column_types::{AuthMethod, AuthorityRef, BoolOrBytes, CredentialRef};
use crate::spec::opal::locking::*;

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
    pub sp_specific: SPSpecific,
}

#[derive(AsArray)]
#[as_array_traits(GenericTable)]
pub struct SPSpecific {
    pub locking: LockingTable,
}

impl LockingSP {
    pub fn new() -> Self {
        Self::default()
    }
}

impl SecurityProvider for LockingSP {
    fn get_table(&self, table: TableUID) -> Option<&dyn GenericTable> {
        let basic = self.basic_sp.as_array().into_iter().find(|table_| table_.uid() == table);
        let specific = self.sp_specific.as_array().into_iter().find(|table_| table_.uid() == table);
        basic.or(specific)
    }

    fn get_table_mut(&mut self, table: TableUID) -> Option<&mut dyn GenericTable> {
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
        let authorities = preconfig_authority();
        let c_pin = preconfig_c_pin();
        let locking = preconfig_locking();
        Self { basic_sp: BasicSP { authorities, c_pin }, sp_specific: SPSpecific { locking } }
    }
}

fn preconfig_authority() -> AuthorityTable {
    let mut authorities = AuthorityTable::new();
    for i in 1..=4 {
        let admin = Authority {
            name: Some(format!("Admin{}", i).into()),
            enabled: false.into(),
            operation: AuthMethod::Password.into(),
            credential: Some(CredentialRef::new_other(c_pin::ADMIN.nth(i).unwrap())),
            ..Authority::new(authority::ADMIN.nth(i).unwrap())
        };
        authorities.insert(admin.uid, admin);
    }
    for i in 1..=8 {
        let admin = Authority {
            name: Some(format!("User{}", i).into()),
            enabled: false.into(),
            operation: AuthMethod::Password.into(),
            credential: Some(CredentialRef::new_other(c_pin::USER.nth(i).unwrap())),
            ..Authority::new(authority::USER.nth(i).unwrap())
        };
        authorities.insert(admin.uid, admin);
    }
    authorities
}

fn preconfig_c_pin() -> CPINTable {
    let mut c_pin = CPINTable::new();
    for i in 1..=4 {
        let admin = CPIN { pin: Some("8965823nz987gt346".into()), ..CPIN::new(c_pin::ADMIN.nth(i).unwrap()) };
        c_pin.insert(admin.uid, admin);
    }
    for i in 1..=8 {
        let user = CPIN { pin: Some("8965823nz987gt346".into()), ..CPIN::new(c_pin::USER.nth(i).unwrap()) };
        c_pin.insert(user.uid, user);
    }
    c_pin
}

fn preconfig_locking() -> LockingTable {
    let mut locking = LockingTable::new();

    let global_range = LockingRange::new(locking::GLOBAL_RANGE);
    locking.insert(global_range.uid, global_range);

    for i in 1..=8 {
        let range = LockingRange::new(locking::RANGE.nth(i).unwrap());
        locking.insert(range.uid, range);
    }

    locking
}
