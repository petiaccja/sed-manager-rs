use crate::messaging::discovery::Feature;
use crate::messaging::types::SPRef;
use crate::rpc::Error as RPCError;
use crate::specification;
use crate::{
    messaging::discovery::{Discovery, FeatureCode, FeatureDescriptor},
    tper::TPer,
};

#[allow(unused)]
const SUPPORTED_SSCS: [FeatureCode; 8] = [
    FeatureCode::Enterprise, // How?
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
        FeatureCode::Enterprise => Ok(specification::enterprise::admin::sp::ADMIN.into()),
        FeatureCode::OpalV1 => Ok(specification::opal::admin::sp::ADMIN.into()),
        FeatureCode::OpalV2 => Ok(specification::opal::admin::sp::ADMIN.into()),
        FeatureCode::Opalite => Ok(specification::opalite::admin::sp::ADMIN.into()),
        FeatureCode::PyriteV1 => Ok(specification::pyrite::admin::sp::ADMIN.into()),
        FeatureCode::PyriteV2 => Ok(specification::pyrite::admin::sp::ADMIN.into()),
        FeatureCode::Ruby => Ok(specification::ruby::admin::sp::ADMIN.into()),
        FeatureCode::KeyPerIO => Ok(specification::kpio::admin::sp::ADMIN.into()),
        _ => Err(Error::IncompatibleSSC),
    }
}

pub fn take_ownership(tper: &TPer) -> Result<(), Error> {
    let discovery = tper.discover()?;
    let default_ssc = get_default_ssc(&discovery)?;
    let _admin_sp = get_admin_sp(default_ssc.feature_code())?;
    Ok(())
}
