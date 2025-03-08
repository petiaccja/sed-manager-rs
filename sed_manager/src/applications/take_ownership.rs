use crate::applications::utility::get_admin_sp;
use crate::messaging::discovery::Discovery;
use crate::spec;
use crate::spec::column_types::Password;
use crate::spec::objects::CPIN;
use crate::tper::TPer;

use super::error::Error;
use super::with_session::with_session;

pub fn is_taking_ownership_supported(_discovery: &Discovery) -> bool {
    true
}

pub async fn take_ownership(tper: &TPer, new_password: &[u8]) -> Result<(), Error> {
    use spec::core::authority;
    use spec::opal::admin::c_pin;

    let discovery = tper.discover().await?;
    let ssc = discovery.get_primary_ssc().ok_or(Error::NoAvailableSSC)?;
    let admin_sp = get_admin_sp(ssc.feature_code())?;

    let anybody_session = tper.start_session(admin_sp, None, None).await?;
    let msid_password: Password = with_session!(session = anybody_session => {
        session.get(c_pin::MSID.as_uid(), CPIN::PIN).await
    })?;
    let sid_session = tper.start_session(admin_sp, Some(authority::SID), Some(&msid_password)).await?;
    with_session!(session = sid_session => {
        session.set(c_pin::SID.as_uid(), CPIN::PIN, new_password).await
    })?;

    Ok(())
}

pub async fn verify_ownership(tper: &TPer, sid_password: &[u8]) -> Result<bool, Error> {
    use spec::core::authority;
    let discovery = tper.discover().await?;
    let ssc = discovery.get_primary_ssc().ok_or(Error::NoAvailableSSC)?;
    let admin_sp = get_admin_sp(ssc.feature_code())?;
    with_session!(session = tper.start_session(admin_sp, Some(authority::SID), Some(sid_password)).await? => {});
    Ok(true)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use crate::{fake_device::FakeDevice, tper::TPer};

    use super::*;

    #[tokio::test]
    async fn take_ownership_success() -> Result<(), Error> {
        let new_password = "macilaci".as_bytes();
        let device = Arc::new(FakeDevice::new());
        let tper = TPer::new_on_default_com_id(device)?;
        take_ownership(&tper, new_password).await?;
        assert!(verify_ownership(&tper, new_password).await?);
        Ok(())
    }

    #[tokio::test]
    async fn take_ownership_already_taken() -> Result<(), Error> {
        let new_password = "macilaci".as_bytes();
        let device = Arc::new(FakeDevice::new());
        let tper = TPer::new_on_default_com_id(device)?;
        take_ownership(&tper, new_password).await?;
        assert!(take_ownership(&tper, "zsiroskenyer".as_bytes()).await.is_err());
        Ok(())
    }
}
