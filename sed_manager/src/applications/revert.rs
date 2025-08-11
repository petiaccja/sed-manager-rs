//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::applications::utility::get_admin_sp;
use crate::messaging::discovery::Discovery;
use crate::spec::column_types::{AuthorityRef, SPRef};
use crate::tper::TPer;

use super::error::Error;

pub fn is_revert_supported(_discovery: &Discovery) -> bool {
    true
}

pub async fn revert(tper: &TPer, authority: AuthorityRef, password: &[u8], sp: SPRef) -> Result<(), Error> {
    let discovery = tper.discover().await?;
    let ssc = discovery.get_primary_ssc().ok_or(Error::NoAvailableSSC)?;
    let admin_sp = get_admin_sp(ssc.feature_code())?;

    let session = tper.start_session(admin_sp, Some(authority), Some(password)).await?;
    let result = session.revert(sp).await;
    if sp == admin_sp {
        session.abort_session();
    } else {
        let _ = session.end_session().await;
    }
    Ok(result?)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::applications::test_fixtures::{make_activated_device, SID_PASSWORD};
    use crate::fake_device::{data::object_table::CPINTable, FakeDevice};
    use crate::rpc::TokioRuntime;
    use crate::spec;
    use crate::spec::column_types::LifeCycleState;
    use crate::spec::table_id;
    use crate::tper::TPer;

    use super::*;

    pub fn is_admin_in_factory_state(device: &FakeDevice) -> bool {
        device.with_tper(|tper| {
            let admin_sp = tper.ssc.get_admin_sp().unwrap();
            let c_pin_table: &CPINTable = admin_sp.get_object_table_specific(table_id::C_PIN).unwrap();
            let sid_c_pin = c_pin_table.get(&spec::opal::admin::c_pin::SID).unwrap();
            let msid_c_pin = c_pin_table.get(&spec::opal::admin::c_pin::MSID).unwrap();
            sid_c_pin.pin == msid_c_pin.pin
        })
    }

    pub fn is_locking_in_factory_state(device: &FakeDevice) -> bool {
        device.with_tper(|tper| {
            println!("{:?}", tper.ssc.get_life_cycle_state(spec::opal::admin::sp::LOCKING));
            tper.ssc.get_life_cycle_state(spec::opal::admin::sp::LOCKING) == Ok(LifeCycleState::ManufacturedInactive)
        })
    }

    #[tokio::test]
    async fn revert_success_admin() -> Result<(), Error> {
        let runtime = Arc::new(TokioRuntime::new());
        let device = Arc::new(make_activated_device());
        let tper = TPer::new_on_default_com_id(device.clone(), runtime)?;
        assert!(!is_admin_in_factory_state(&*device));
        assert!(!is_locking_in_factory_state(&*device));
        revert(&tper, spec::core::authority::SID, SID_PASSWORD.as_bytes(), spec::opal::admin::sp::ADMIN).await?;
        assert!(is_admin_in_factory_state(&*device));
        assert!(is_locking_in_factory_state(&*device));
        Ok(())
    }

    #[tokio::test]
    async fn revert_success_locking() -> Result<(), Error> {
        let runtime = Arc::new(TokioRuntime::new());
        let device = Arc::new(make_activated_device());
        let tper = TPer::new_on_default_com_id(device.clone(), runtime)?;
        assert!(!is_locking_in_factory_state(&*device));
        revert(&tper, spec::core::authority::SID, SID_PASSWORD.as_bytes(), spec::opal::admin::sp::LOCKING).await?;
        assert!(is_locking_in_factory_state(&*device));
        Ok(())
    }
}
