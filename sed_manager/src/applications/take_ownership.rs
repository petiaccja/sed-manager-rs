//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::applications::utility::get_admin_sp;
use crate::messaging::discovery::{Discovery, LockingDescriptor};
use crate::spec;
use crate::spec::column_types::Password;
use crate::spec::objects::CPIN;
use crate::tper::TPer;

use super::error::Error;

pub fn is_taking_ownership_supported(discovery: &Discovery) -> bool {
    if let Some(locking_desc) = discovery.get::<LockingDescriptor>() {
        // We cannot reliably tell from the discovery if someone has already taken
        // ownership of the device. However, if locking is enabled, someone definitely has
        // taken ownership, therefore taking ownership is not supported anymore.
        !locking_desc.locking_enabled
    } else {
        // Otherwise, taking ownership is always supported.
        true
    }
}

pub async fn take_ownership(tper: &TPer, new_password: &[u8]) -> Result<(), Error> {
    use spec::core::authority;
    use spec::opal::admin::c_pin;

    let discovery = tper.discover().await?;
    let ssc = discovery.get_primary_ssc().ok_or(Error::NoAvailableSSC)?;
    let admin_sp = get_admin_sp(ssc.feature_code())?;

    let anybody_session = tper.start_session(admin_sp, None, None).await?;
    let msid_password: Password =
        anybody_session.with(async |session| session.get(c_pin::MSID.as_uid(), CPIN::PIN).await).await?;
    let sid_session = tper.start_session(admin_sp, Some(authority::SID), Some(&msid_password)).await?;
    sid_session
        .with(async |session| session.set(c_pin::SID.as_uid(), CPIN::PIN, new_password).await)
        .await?;

    Ok(())
}

pub async fn verify_ownership(tper: &TPer, sid_password: &[u8]) -> Result<bool, Error> {
    use spec::core::authority;
    let discovery = tper.discover().await?;
    let ssc = discovery.get_primary_ssc().ok_or(Error::NoAvailableSSC)?;
    let admin_sp = get_admin_sp(ssc.feature_code())?;
    let _ = tper.start_session(admin_sp, Some(authority::SID), Some(sid_password)).await?.end_session().await;
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{fake_device::FakeDevice, rpc::TokioRuntime, tper::TPer};

    use super::*;

    #[tokio::test]
    async fn take_ownership_success() -> Result<(), Error> {
        let new_password = "macilaci".as_bytes();
        let runtime = Arc::new(TokioRuntime::new());
        let device = Arc::new(FakeDevice::new());
        let tper = TPer::new_on_default_com_id(device, runtime)?;
        take_ownership(&tper, new_password).await?;
        assert!(verify_ownership(&tper, new_password).await?);
        Ok(())
    }

    #[tokio::test]
    async fn take_ownership_already_taken() -> Result<(), Error> {
        let new_password = "macilaci".as_bytes();
        let runtime = Arc::new(TokioRuntime::new());
        let device = Arc::new(FakeDevice::new());
        let tper = TPer::new_on_default_com_id(device, runtime)?;
        take_ownership(&tper, new_password).await?;
        assert!(take_ownership(&tper, "zsiroskenyer".as_bytes()).await.is_err());
        Ok(())
    }
}
