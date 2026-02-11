//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::spec::{column_types::LifeCycleState, objects::SP, opal::admin::*};
use crate::virtual_device::data::object_table::SPTable;

pub fn preconfig_sp() -> SPTable {
    let items = [
        SP {
            uid: sp::ADMIN,
            name: "Admin".into(),
            life_cycle_state: LifeCycleState::Manufactured,
            ..Default::default()
        },
        SP {
            uid: sp::LOCKING,
            name: "Locking".into(),
            life_cycle_state: LifeCycleState::ManufacturedInactive,
            ..Default::default()
        },
    ];

    items.into_iter().collect()
}
