use crate::messaging::discovery::Feature;
use crate::messaging::discovery::{Discovery, FeatureCode, FeatureDescriptor};
use crate::rpc::Error as RPCError;
use crate::spec;
use crate::spec::column_types::{Password, SPRef};
use crate::tper::TPer;

#[allow(unused)]
const SUPPORTED_SSCS: [FeatureCode; 8] = [
    FeatureCode::Enterprise,
    FeatureCode::OpalV1,
    FeatureCode::OpalV2,
    FeatureCode::Opalite,
    FeatureCode::PyriteV1,
    FeatureCode::PyriteV2,
    FeatureCode::Ruby,
    FeatureCode::KeyPerIO,
];

pub enum Error {
    RPCError(RPCError),
    IncompatibleSSC,
    NoAvailableSSC,
    AlreadyOwned,
}

impl From<RPCError> for Error {
    fn from(value: RPCError) -> Self {
        Self::RPCError(value)
    }
}

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

/// Run block of code and close session asynchronously afterwards.
///
/// While the session would be closed by [`Drop`] without blocking, it might
/// take a while until the protocol thread actually closes the session.
/// This can lead to weird issues like SPBusy when opening the next session.
/// This macro ensures the session really is closed before returning.
macro_rules! with_session {
    ($id:ident = $session:expr => $block:expr) => {{
        let $id = $session;
        let result = async { $block }.await;
        let _ = $id.end_session().await;
        result
    }};
    ($id:ident => $block:expr) => {{
        let result = async { $block }.await;
        let _ = $id.end_session().await;
        result
    }};
}

pub async fn take_ownership(tper: &TPer, new_password: Password) -> Result<(), Error> {
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
