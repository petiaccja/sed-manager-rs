use crate::applications::utility::get_locking_admins;
use crate::messaging::discovery::{Discovery, Feature};
use crate::spec::column_types::LifeCycleState;
use crate::spec::core;
use crate::tper::TPer;

use super::error::Error;
use super::utility::{get_admin_sp, get_default_ssc, get_locking_admin_c_pins, get_locking_sp};
use super::with_session::with_session;

pub async fn is_activating_locking_supported(_discovery: &Discovery) -> bool {
    true
}

pub async fn activate_locking(
    tper: &TPer,
    sid_password: &[u8],
    new_admin1_password: Option<&[u8]>,
) -> Result<(), Error> {
    let discovery = tper.discover().await?;
    let default_ssc = get_default_ssc(&discovery)?;
    let admin_sp = get_admin_sp(default_ssc.feature_code())?;
    let locking_sp = get_locking_sp(default_ssc.feature_code())?;

    // Activate the locking SP.
    let admin_session = tper.start_session(admin_sp, Some(core::authority::SID), Some(sid_password)).await?;
    with_session!(session = admin_session => {
        let life_cycle_state : LifeCycleState = session.get(locking_sp.as_uid(), 0x06).await?;
        if life_cycle_state != LifeCycleState::ManufacturedInactive {
            return Err(Error::AlreadyActivated);
        }
        session.activate(locking_sp).await?;
        Ok::<(), Error>(())
    })?;

    // Change Admin1 PIN on the locking SP.
    let admin1 = get_locking_admins(default_ssc.feature_code()).ok().map(|uids| uids.nth(1)).flatten();
    let admin1_c_pin = get_locking_admin_c_pins(default_ssc.feature_code()).ok().map(|uids| uids.nth(1)).flatten();
    if let (Some(admin1_pw), Some(admin1), Some(admin1_c_pin)) = (new_admin1_password, admin1, admin1_c_pin) {
        let locking_session = tper.start_session(locking_sp, Some(admin1), Some(sid_password)).await?;
        with_session!(session = locking_session => {
            session.set(admin1_c_pin.as_uid(), 0x03, admin1_pw).await
        })?;
    }

    Ok(())
}

pub async fn verify_locking_activation(tper: &TPer, admin1_password: Option<&[u8]>) -> Result<bool, Error> {
    let discovery = tper.discover().await?;
    let default_ssc = get_default_ssc(&discovery)?;
    let locking_sp = get_locking_sp(default_ssc.feature_code())?;
    let admin1 = get_locking_admins(default_ssc.feature_code()).ok().map(|uids| uids.nth(1)).flatten();
    let password = admin1_password.filter(|_| admin1.is_some());
    with_session!(session = tper.start_session(locking_sp, admin1, password).await? => {});
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{
        fake_device::{FakeDevice, MSID_PASSWORD},
        tper::TPer,
    };

    use super::*;

    #[tokio::test]
    async fn activate_locking_success_no_pw() -> Result<(), Error> {
        let sid_password = MSID_PASSWORD.as_bytes();
        let new_password = None;
        let device = Arc::new(FakeDevice::new());
        let tper = TPer::new_on_default_com_id(device)?;
        activate_locking(&tper, sid_password, new_password).await?;
        verify_locking_activation(&tper, Some(sid_password)).await?;
        Ok(())
    }

    #[tokio::test]
    async fn activate_locking_success_with_pw() -> Result<(), Error> {
        let sid_password = MSID_PASSWORD.as_bytes();
        let new_password = Some("macilaci".as_bytes());
        let device = Arc::new(FakeDevice::new());
        let tper = TPer::new_on_default_com_id(device)?;
        activate_locking(&tper, sid_password, new_password).await?;
        verify_locking_activation(&tper, new_password).await?;
        Ok(())
    }

    #[tokio::test]
    async fn activate_locking_already_locked() -> Result<(), Error> {
        let sid_password = MSID_PASSWORD.as_bytes();
        let new_password = Some("macilaci".as_bytes());
        let device = Arc::new(FakeDevice::new());
        let tper = TPer::new_on_default_com_id(device)?;
        activate_locking(&tper, sid_password, new_password).await?;
        assert!(activate_locking(&tper, sid_password, new_password).await.is_err());
        Ok(())
    }
}
