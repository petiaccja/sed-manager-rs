use crate::fake_device::data::object_table::SPTable;
use crate::spec::{column_types::LifeCycleState, objects::SP, opal::admin::*};

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
