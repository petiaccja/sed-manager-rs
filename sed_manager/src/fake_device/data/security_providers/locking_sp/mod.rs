//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use as_array::AsArray;

use crate::fake_device::data::access_control_table::AccessControlTable;
use crate::fake_device::data::byte_table::ByteTable;
use crate::fake_device::data::object_table::{GenericTable, KAES256Table, LockingTable, MBRControlTable};
use crate::messaging::uid::{TableUID, UID};
use crate::messaging::value::Bytes;
use crate::rpc::MethodStatus;
use crate::spec::column_types::{ACERef, AuthorityRef, BoolOrBytes, CredentialRef, KAES256Ref, Key256, MethodRef};
use crate::spec::table_id;

use super::basic_sp::BasicSP;
use super::security_provider::SecurityProvider;

mod preconfig_access_control;
mod preconfig_ace;
mod preconfig_authority;
mod preconfig_c_pin;
mod preconfig_k_aes_256;
mod preconfig_locking;
mod preconfig_mbr_control;
mod preconfig_table;

const ADMIN_IDX: core::ops::RangeInclusive<u64> = 1_u64..=4_u64;
const USER_IDX: core::ops::RangeInclusive<u64> = 1_u64..=8_u64;
const RANGE_IDX: core::ops::RangeInclusive<u64> = 1_u64..=8_u64;

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
    pub access_control: AccessControlTable,
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

    fn get_acl(&self, invoking_id: UID, method_id: MethodRef) -> Result<Vec<ACERef>, MethodStatus> {
        let entry = self.access_control.get(&invoking_id, &method_id).ok_or(MethodStatus::InvalidParameter)?;
        Ok(entry.acl.0.clone())
    }
}

impl Default for LockingSP {
    fn default() -> Self {
        Self {
            access_control: preconfig_access_control::preconfig_access_control(),
            basic_sp: BasicSP {
                table: preconfig_table::preconfig_table(),
                ace: preconfig_ace::preconfig_ace(),
                authority: preconfig_authority::preconfig_authority(),
                c_pin: preconfig_c_pin::preconfig_c_pin(),
            },
            sp_specific: SPSpecific {
                locking: preconfig_locking::preconfig_locking(),
                k_aes_256: preconfig_k_aes_256::preconfig_k_aes_256(),
                mbr_control: preconfig_mbr_control::preconfig_mbr_control(),
            },
            byte_tables: ByteTables { mbr: ByteTable::new(MBR_SIZE as usize) },
        }
    }
}
