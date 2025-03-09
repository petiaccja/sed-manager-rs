use as_array::AsArray;

use crate::fake_device::data::byte_table::ByteTable;
use crate::fake_device::data::table::{
    AuthorityTable, CPINTable, GenericTable, KAES256Table, LockingTable, MBRControlTable, TableTable,
};
use crate::messaging::uid::{ObjectUID, TableUID};
use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::spec::column_types::{AuthMethod, AuthorityRef, BoolOrBytes, CredentialRef, KAES256Ref, Key256, TableKind};
use crate::spec::objects::{Authority, LockingRange, MBRControl, TableDesc, CPIN, KAES256};
use crate::spec::opal::locking::*;
use crate::spec::{self, table_id};

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
    pub byte_tables: ByteTables,
}

#[derive(AsArray)]
#[as_array_traits(GenericTable)]
pub struct SPSpecific {
    pub locking: LockingTable,
    pub k_aes_256: KAES256Table,
    pub mbr_control: MBRControlTable,
}

pub struct ByteTables {
    mbr: ByteTable,
}

impl LockingSP {
    pub fn new() -> Self {
        Self::default()
    }
}

const MBR_SIZE: u32 = 0x08000000;

impl SecurityProvider for LockingSP {
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

    fn get_byte_table(&self, table: TableUID) -> Option<&ByteTable> {
        match table {
            table_id::MBR => Some(&self.byte_tables.mbr),
            _ => None,
        }
    }

    fn get_byte_table_mut(&mut self, table: TableUID) -> Option<&mut ByteTable> {
        match table {
            table_id::MBR => Some(&mut self.byte_tables.mbr),
            _ => None,
        }
    }

    fn authenticate(&self, authority_id: AuthorityRef, proof: Option<Bytes>) -> Result<BoolOrBytes, MethodStatus> {
        self.basic_sp.authenticate(authority_id, proof)
    }

    fn gen_key(
        &mut self,
        credential_id: CredentialRef,
        _public_exponent: Option<u64>,
        _pin_length: Option<u16>,
    ) -> Result<(), MethodStatus> {
        if let Ok(k_aes_256_id) = KAES256Ref::try_new_other(credential_id) {
            if let Some(object) = self.sp_specific.k_aes_256.get_mut(&k_aes_256_id) {
                object.key = Key256::Bytes64([0xFF; 64]);
                Ok(())
            } else {
                Err(MethodStatus::InvalidParameter)
            }
        } else {
            Err(MethodStatus::InvalidParameter)
        }
    }
}

impl Default for LockingSP {
    fn default() -> Self {
        let table = preconfig_table();
        let authorities = preconfig_authority();
        let c_pin = preconfig_c_pin();
        let locking = preconfig_locking();
        let k_aes_256 = preconfig_k_aes_256();
        let mbr_control = preconfig_mbr_control();
        let mbr = ByteTable::new(MBR_SIZE as usize);
        Self {
            basic_sp: BasicSP { table, authorities, c_pin },
            sp_specific: SPSpecific { locking, k_aes_256, mbr_control },
            byte_tables: ByteTables { mbr },
        }
    }
}

fn preconfig_mbr_control() -> MBRControlTable {
    let mbr_control = MBRControl { ..Default::default() };
    let mut mbr_control_table = MBRControlTable::new();
    mbr_control_table.insert(mbr_control.uid, mbr_control);
    mbr_control_table
}

fn preconfig_table() -> TableTable {
    let mbr_control = TableDesc {
        uid: spec::core::table::MBR_CONTROL,
        name: "MBRControl".into(),
        kind: TableKind::Object,
        ..Default::default()
    };
    let mbr = TableDesc {
        uid: spec::core::table::MBR,
        name: "MBR".into(),
        kind: TableKind::Byte,
        rows: MBR_SIZE,
        ..Default::default()
    };

    let mut table = TableTable::new();
    table.insert(mbr_control.uid, mbr_control);
    table.insert(mbr.uid, mbr);
    table
}

fn preconfig_authority() -> AuthorityTable {
    let admins = (1..=4).map(|index| Authority {
        uid: authority::ADMIN.nth(index).unwrap(),
        name: format!("Admin{}", index).into(),
        is_class: false,
        enabled: (index == 1),
        operation: AuthMethod::Password,
        credential: CredentialRef::new_other(c_pin::ADMIN.nth(index).unwrap()),
        ..Default::default()
    });
    let users = (1..=8).map(|index| Authority {
        uid: authority::USER.nth(index).unwrap(),
        name: format!("User{}", index).into(),
        is_class: false,
        enabled: false,
        operation: AuthMethod::Password,
        credential: CredentialRef::new_other(c_pin::USER.nth(index).unwrap()),
        ..Default::default()
    });

    let mut authorities = AuthorityTable::new();
    for authority in admins {
        authorities.insert(authority.uid, authority);
    }
    for authority in users {
        authorities.insert(authority.uid, authority);
    }
    authorities
}

fn preconfig_c_pin() -> CPINTable {
    let admins = (1..=4).map(|index| CPIN {
        uid: c_pin::ADMIN.nth(index).unwrap(),
        pin: "8965823nz987gt346".into(),
        ..Default::default()
    });
    let users = (1..=8).map(|index| CPIN {
        uid: c_pin::USER.nth(index).unwrap(),
        pin: "8965823nz987gt346".into(),
        ..Default::default()
    });

    let mut c_pin = CPINTable::new();
    for pin in admins {
        c_pin.insert(pin.uid, pin);
    }
    for pin in users {
        c_pin.insert(pin.uid, pin);
    }
    c_pin
}

fn preconfig_locking() -> LockingTable {
    let global_range = LockingRange {
        uid: locking::GLOBAL_RANGE,
        active_key: ObjectUID::new_other(k_aes_256::GLOBAL_RANGE_KEY),
        ..Default::default()
    };

    let ranges = (1..=8).map(|index| LockingRange {
        uid: locking::RANGE.nth(index).unwrap(),
        active_key: ObjectUID::new_other(k_aes_256::RANGE_KEY.nth(index).unwrap()),
        ..Default::default()
    });

    let mut locking = LockingTable::new();
    locking.insert(global_range.uid, global_range);
    for range in ranges {
        locking.insert(range.uid, range);
    }
    locking
}

fn preconfig_k_aes_256() -> KAES256Table {
    let global_range = KAES256 { uid: k_aes_256::GLOBAL_RANGE_KEY, ..Default::default() };
    let ranges = (1..=8).map(|index| KAES256 { uid: k_aes_256::RANGE_KEY.nth(index).unwrap(), ..Default::default() });

    let mut k_aes_256 = KAES256Table::new();
    k_aes_256.insert(global_range.uid, global_range);
    for range in ranges {
        k_aes_256.insert(range.uid, range);
    }
    k_aes_256
}
