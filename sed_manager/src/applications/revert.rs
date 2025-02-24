use crate::applications::utility::{get_admin_sp, get_default_ssc};
use crate::messaging::discovery::{Discovery, Feature};
use crate::spec;
use crate::spec::column_types::{AuthorityRef, Password, SPRef};
use crate::tper::TPer;

use super::error::Error;
use super::with_session::with_session;

pub async fn is_revert_supported(_discovery: &Discovery) -> bool {
    true
}

pub async fn revert(tper: &TPer, authority: AuthorityRef, password: &[u8], sp: SPRef) -> Result<(), Error> {
    let discovery = tper.discover()?;
    let default_ssc = get_default_ssc(&discovery)?;
    let admin_sp = get_admin_sp(default_ssc.feature_code())?;

    let session = tper.start_session(admin_sp, Some(authority), Some(password)).await?;
    with_session!(session => {
        session.revert(sp).await
    })?;

    Ok(())
}

pub async fn verify_reverted(tper: &TPer) -> Result<bool, Error> {
    use spec::core::authority;
    use spec::opal::admin::c_pin;

    let discovery = tper.discover()?;
    let default_ssc = get_default_ssc(&discovery)?;
    let admin_sp = get_admin_sp(default_ssc.feature_code())?;

    let anybody_session = tper.start_session(admin_sp, None, None).await?;
    let msid_password: Password = with_session!(session = anybody_session => {
        session.get(c_pin::MSID.as_uid(), 3).await
    })?;
    with_session!(session = tper.start_session(admin_sp, Some(authority::SID), Some(&msid_password)).await? => {});
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{fake_device::FakeDevice, tper::TPer};

    use super::*;

    #[tokio::test]
    async fn revert_success_admin() -> Result<(), Error> {
        let sid_password = "macilaci";
        let device = Arc::new(FakeDevice::new());
        {
            let controller = device.controller();
            let mut controller = controller.lock().unwrap();
            let sid = controller.admin_sp.basic_sp.c_pin.get_mut(&spec::opal::admin::c_pin::SID).unwrap();
            sid.pin = Some(sid_password.into());
        };
        let tper = TPer::new_on_default_com_id(device)?;
        revert(&tper, spec::core::authority::SID, sid_password.as_bytes(), spec::opal::admin::sp::ADMIN).await?;
        assert!(verify_reverted(&tper).await?);
        Ok(())
    }

    #[tokio::test]
    async fn revert_success_locking() -> Result<(), Error> {
        let sid_password = "macilaci";
        let device = Arc::new(FakeDevice::new());
        {
            let controller = device.controller();
            let mut controller = controller.lock().unwrap();
            let sid = controller.admin_sp.basic_sp.c_pin.get_mut(&spec::opal::admin::c_pin::SID).unwrap();
            sid.pin = Some(sid_password.into());
        };
        let tper = TPer::new_on_default_com_id(device)?;
        revert(&tper, spec::core::authority::SID, sid_password.as_bytes(), spec::opal::admin::sp::LOCKING).await?;
        //assert!(verify_reverted(&tper).await?);
        Ok(())
    }
}
