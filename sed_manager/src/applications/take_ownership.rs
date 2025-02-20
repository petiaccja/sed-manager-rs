use crate::messaging::discovery::Feature;
use crate::messaging::discovery::{Discovery, FeatureCode, FeatureDescriptor};
use crate::spec;
use crate::spec::column_types::{Password, SPRef};
use crate::tper::TPer;

use super::error::Error;
use super::with_session::with_session;

fn get_default_ssc(discovery: &Discovery) -> Result<&FeatureDescriptor, Error> {
    discovery
        .descriptors
        .iter()
        .find(|desc| desc.security_subsystem_class().is_some())
        .ok_or(Error::NoAvailableSSC)
}

fn get_admin_sp(ssc: FeatureCode) -> Result<SPRef, Error> {
    match ssc {
        FeatureCode::Enterprise => Ok(spec::enterprise::admin::sp::ADMIN),
        FeatureCode::OpalV1 => Ok(spec::opal::admin::sp::ADMIN),
        FeatureCode::OpalV2 => Ok(spec::opal::admin::sp::ADMIN),
        FeatureCode::Opalite => Ok(spec::opalite::admin::sp::ADMIN),
        FeatureCode::PyriteV1 => Ok(spec::pyrite::admin::sp::ADMIN),
        FeatureCode::PyriteV2 => Ok(spec::pyrite::admin::sp::ADMIN),
        FeatureCode::Ruby => Ok(spec::ruby::admin::sp::ADMIN),
        FeatureCode::KeyPerIO => Ok(spec::kpio::admin::sp::ADMIN),
        _ => Err(Error::IncompatibleSSC),
    }
}

pub async fn take_ownership(tper: &TPer, new_password: &[u8]) -> Result<(), Error> {
    use spec::core::authority;
    use spec::opal::admin::c_pin;

    let discovery = tper.discover()?;
    let default_ssc = get_default_ssc(&discovery)?;
    let admin_sp = get_admin_sp(default_ssc.feature_code())?;

    let anybody_session = tper.start_session(admin_sp, None, None).await?;
    let msid_password: Password = with_session!(session = anybody_session => {
        session.get(c_pin::MSID.as_uid(), 3).await
    })?;
    let sid_session = tper.start_session(admin_sp, Some(authority::SID), Some(&msid_password)).await?;
    with_session!(session = sid_session => {
        session.set(c_pin::SID.as_uid(), 3, new_password).await
    })?;

    Ok(())
}

pub async fn verify_ownership(tper: &TPer, sid_password: &[u8]) -> Result<bool, Error> {
    use spec::core::authority;
    use spec::enterprise::admin::sp;
    with_session!(session = tper.start_session(sp::ADMIN, Some(authority::SID), Some(sid_password)).await? => {});
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
        verify_ownership(&tper, new_password).await?;
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
