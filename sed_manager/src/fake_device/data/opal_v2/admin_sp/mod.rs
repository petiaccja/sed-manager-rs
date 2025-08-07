//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::fake_device::data::object_table::GenericTable;
use crate::fake_device::data::security_provider::SecurityProvider;

mod preconfig_access_control;
mod preconfig_ace;
mod preconfig_authority;
mod preconfig_c_pin;
mod preconfig_sp;
mod preconfig_table;

const ADMIN_IDX: core::ops::RangeInclusive<u64> = 1_u64..=4_u64;

pub fn new_admin_sp() -> SecurityProvider {
    let access_control = preconfig_access_control::preconfig_access_control();
    let object_tables = [
        Box::new(preconfig_table::preconfig_table()) as Box<dyn GenericTable>,
        Box::new(preconfig_ace::preconfig_ace()) as Box<dyn GenericTable>,
        Box::new(preconfig_authority::preconfig_authority()) as Box<dyn GenericTable>,
        Box::new(preconfig_c_pin::preconfig_c_pin()) as Box<dyn GenericTable>,
        Box::new(preconfig_sp::preconfig_sp()) as Box<dyn GenericTable>,
    ];
    SecurityProvider {
        access_control,
        object_tables: object_tables.into_iter().map(|x| (x.uid(), x)).collect(),
        byte_tables: [].into_iter().collect(),
    }
}

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
