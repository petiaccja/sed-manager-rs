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
        FeatureCode::Enterprise => Ok(spec::enterprise::admin::sp::ADMIN.into()),
        FeatureCode::OpalV1 => Ok(spec::opal::admin::sp::ADMIN.into()),
        FeatureCode::OpalV2 => Ok(spec::opal::admin::sp::ADMIN.into()),
        FeatureCode::Opalite => Ok(spec::opalite::admin::sp::ADMIN.into()),
        FeatureCode::PyriteV1 => Ok(spec::pyrite::admin::sp::ADMIN.into()),
        FeatureCode::PyriteV2 => Ok(spec::pyrite::admin::sp::ADMIN.into()),
        FeatureCode::Ruby => Ok(spec::ruby::admin::sp::ADMIN.into()),
        FeatureCode::KeyPerIO => Ok(spec::kpio::admin::sp::ADMIN.into()),
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
        session.get(c_pin::MSID, 3).await
    })?;
    let sid_session = tper.start_session(admin_sp, Some(authority::SID.into()), Some(&msid_password)).await?;
    with_session!(session = sid_session => {
        session.set(c_pin::SID, 3, new_password).await
    })?;

    Ok(())
}

pub async fn verify_ownership(tper: &TPer, sid_password: &[u8]) -> Result<bool, Error> {
    use spec::core::authority;
    use spec::enterprise::admin::sp;
    let _session = tper.start_session(sp::ADMIN.into(), Some(authority::SID.into()), Some(sid_password)).await?;
    Ok(true)
}
