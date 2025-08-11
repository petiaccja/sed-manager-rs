//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

mod admin_sp;
mod locking_sp;

pub use admin_sp::new_admin_sp;
pub use locking_sp::new_locking_sp;

use crate::fake_device::data::security_provider::SecurityProvider;
use crate::fake_device::data::SecuritySubsystemClass;
use crate::spec::column_types::SPRef;
use crate::spec::opal;

pub fn new_controller() -> SecuritySubsystemClass {
    SecuritySubsystemClass::new(sp_factory, &[opal::admin::sp::ADMIN, opal::admin::sp::LOCKING])
}

fn sp_factory(sp_ref: SPRef) -> SecurityProvider {
    match sp_ref {
        opal::admin::sp::ADMIN => new_admin_sp(),
        opal::admin::sp::LOCKING => new_locking_sp(),
        _ => unreachable!("this factory should never be passed to a Controller with any other SPs"),
    }
}
