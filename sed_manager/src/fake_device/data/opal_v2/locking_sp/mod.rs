//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::fake_device::data::byte_table::ByteTable;
use crate::fake_device::data::object_table::GenericTable;
use crate::fake_device::data::security_provider::SecurityProvider;
use crate::spec::table_id;

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
const MBR_SIZE: u32 = 0x08000000;

pub fn new_locking_sp() -> SecurityProvider {
    let access_control = preconfig_access_control::preconfig_access_control();
    let object_tables = [
        Box::new(preconfig_table::preconfig_table()) as Box<dyn GenericTable>,
        Box::new(preconfig_ace::preconfig_ace()) as Box<dyn GenericTable>,
        Box::new(preconfig_authority::preconfig_authority()) as Box<dyn GenericTable>,
        Box::new(preconfig_c_pin::preconfig_c_pin()) as Box<dyn GenericTable>,
        Box::new(preconfig_locking::preconfig_locking()) as Box<dyn GenericTable>,
        Box::new(preconfig_k_aes_256::preconfig_k_aes_256()) as Box<dyn GenericTable>,
        Box::new(preconfig_mbr_control::preconfig_mbr_control()) as Box<dyn GenericTable>,
    ];
    let byte_tables = [(table_id::MBR, ByteTable::new(MBR_SIZE as usize))];
    SecurityProvider {
        access_control,
        object_tables: object_tables.into_iter().map(|x| (x.uid(), x)).collect(),
        byte_tables: byte_tables.into_iter().collect(),
    }
}

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
