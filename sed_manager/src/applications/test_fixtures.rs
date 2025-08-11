//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::sync::Arc;

use crate::fake_device::data::object_table::CPINTable;
use crate::fake_device::FakeDevice;
use crate::rpc::TokioRuntime;
use crate::spec::{self, table_id};
use crate::tper::TPer;

pub const SID_PASSWORD: &str = "sid_password";
pub const LOCKING_ADMIN1_PASSWORD: &str = "L_admin1_pw";

pub fn make_factory_device() -> FakeDevice {
    FakeDevice::new()
}

pub fn make_owned_device() -> FakeDevice {
    let device = FakeDevice::new();
    device.with_tper_mut(|tper| {
        let admin_sp = tper.ssc.get_admin_sp_mut().unwrap();
        let c_pin_table: &mut CPINTable = admin_sp.get_object_table_specific_mut(table_id::C_PIN).unwrap();
        let sid_c_pin = c_pin_table.get_mut(&spec::opal::admin::c_pin::SID).unwrap();
        sid_c_pin.pin = SID_PASSWORD.into();
    });
    device
}

pub fn make_activated_device() -> FakeDevice {
    let device = make_owned_device();
    device.with_tper_mut(|tper| {
        tper.ssc.activate_sp(spec::opal::admin::sp::LOCKING).unwrap();
        let locking_sp = tper.ssc.get_sp_mut(spec::opal::admin::sp::LOCKING).unwrap();
        let c_pin_table: &mut CPINTable = locking_sp.get_object_table_specific_mut(table_id::C_PIN).unwrap();
        let admin1_c_pin = c_pin_table.get_mut(&spec::opal::locking::c_pin::ADMIN.nth(1).unwrap()).unwrap();
        admin1_c_pin.pin = LOCKING_ADMIN1_PASSWORD.into();
    });
    device
}

pub fn setup_factory_tper() -> TPer {
    let runtime = Arc::new(TokioRuntime::new());
    let device = Arc::new(make_factory_device());
    TPer::new_on_default_com_id(device, runtime).unwrap()
}

pub fn setup_activated_tper() -> TPer {
    let runtime = Arc::new(TokioRuntime::new());
    let device = Arc::new(make_activated_device());
    TPer::new_on_default_com_id(device, runtime).unwrap()
}
