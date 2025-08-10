//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::applications::utility::get_admin_sp;
use crate::messaging::discovery::Discovery;
use crate::spec;
use crate::spec::column_types::{AuthorityRef, Password, SPRef};
use crate::spec::objects::CPIN;
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

pub async fn verify_reverted(tper: &TPer) -> Result<bool, Error> {
    use spec::core::authority;
    use spec::opal::admin::c_pin;

    let discovery = tper.discover().await?;
    let ssc = discovery.get_primary_ssc().ok_or(Error::NoAvailableSSC)?;
    let admin_sp = get_admin_sp(ssc.feature_code())?;

    let anybody_session = tper.start_session(admin_sp, None, None).await?;
    let msid_password: Password =
        anybody_session.with(async |session| session.get(c_pin::MSID.as_uid(), CPIN::PIN).await).await?;
    let _ = tper.start_session(admin_sp, Some(authority::SID), Some(&msid_password)).await?.end_session().await;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        fake_device::{data::object_table::CPINTable, FakeDevice},
        rpc::TokioRuntime,
        spec::table_id,
        tper::TPer,
    };

    use super::*;

    #[tokio::test]
    async fn revert_success_admin() -> Result<(), Error> {
        let sid_password = "macilaci";
        let runtime = Arc::new(TokioRuntime::new());
        let device = Arc::new(FakeDevice::new());
        {
            let controller = device.controller();
            let mut controller = controller.lock().unwrap();
            let admin_sp = controller.get_sp_mut(spec::opal::admin::sp::ADMIN).unwrap();
            let c_pin_table: &mut CPINTable = admin_sp.get_object_table_specific_mut(table_id::C_PIN).unwrap();
            let sid_c_pin = c_pin_table.get_mut(&spec::opal::admin::c_pin::SID).unwrap();
            sid_c_pin.pin = sid_password.into();
        };
        let tper = TPer::new_on_default_com_id(device, runtime)?;
        revert(&tper, spec::core::authority::SID, sid_password.as_bytes(), spec::opal::admin::sp::ADMIN).await?;
        assert!(verify_reverted(&tper).await?);
        Ok(())
    }

    #[tokio::test]
    async fn revert_success_locking() -> Result<(), Error> {
        let sid_password = "macilaci";
        let runtime = Arc::new(TokioRuntime::new());
        let device = Arc::new(FakeDevice::new());
        {
            let controller = device.controller();
            let mut controller = controller.lock().unwrap();
            let admin_sp = controller.get_sp_mut(spec::opal::admin::sp::ADMIN).unwrap();
            let c_pin_table: &mut CPINTable = admin_sp.get_object_table_specific_mut(table_id::C_PIN).unwrap();
            let sid_c_pin = c_pin_table.get_mut(&spec::opal::admin::c_pin::SID).unwrap();
            sid_c_pin.pin = sid_password.into();
        };
        let tper = TPer::new_on_default_com_id(device, runtime)?;
        revert(&tper, spec::core::authority::SID, sid_password.as_bytes(), spec::opal::admin::sp::LOCKING).await?;
        //assert!(verify_reverted(&tper).await?);
        Ok(())
    }
}
