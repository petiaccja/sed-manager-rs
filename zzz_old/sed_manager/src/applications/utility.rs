//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use crate::messaging::discovery::FeatureCode;
use crate::messaging::uid_range::ObjectUIDRange;
use crate::spec::column_types::{AuthorityRefRange, CPINRefRange, SPRef};
use crate::spec::{self, ObjectLookup};
use crate::tper::{Session, TPer};

use super::error::Error;

pub fn get_admin_sp(ssc: FeatureCode) -> Result<SPRef, Error> {
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

pub fn get_locking_sp(ssc: FeatureCode) -> Result<SPRef, Error> {
    match ssc {
        FeatureCode::Enterprise => Ok(spec::enterprise::admin::sp::LOCKING),
        FeatureCode::OpalV1 => Ok(spec::opal::admin::sp::LOCKING),
        FeatureCode::OpalV2 => Ok(spec::opal::admin::sp::LOCKING),
        FeatureCode::Opalite => Ok(spec::opalite::admin::sp::LOCKING),
        FeatureCode::PyriteV1 => Ok(spec::pyrite::admin::sp::LOCKING),
        FeatureCode::PyriteV2 => Ok(spec::pyrite::admin::sp::LOCKING),
        FeatureCode::Ruby => Ok(spec::ruby::admin::sp::LOCKING),
        FeatureCode::KeyPerIO => Ok(spec::kpio::admin::sp::KEY_PER_IO),
        _ => Err(Error::IncompatibleSSC),
    }
}

pub fn get_locking_admins(ssc: FeatureCode) -> Result<AuthorityRefRange, Error> {
    match ssc {
        FeatureCode::Enterprise => Err(Error::IncompatibleSSC),
        FeatureCode::OpalV1 => Ok(spec::opal::locking::authority::ADMIN),
        FeatureCode::OpalV2 => Ok(spec::opal::locking::authority::ADMIN),
        FeatureCode::Opalite => Ok(ObjectUIDRange::new_count(spec::opalite::locking::authority::ADMIN1, 1, 1)),
        FeatureCode::PyriteV1 => Ok(spec::pyrite::locking::authority::ADMIN),
        FeatureCode::PyriteV2 => Ok(spec::pyrite::locking::authority::ADMIN),
        FeatureCode::Ruby => Ok(spec::ruby::locking::authority::ADMIN),
        FeatureCode::KeyPerIO => Ok(spec::kpio::key_per_io::authority::ADMIN),
        _ => Err(Error::IncompatibleSSC),
    }
}

pub fn get_locking_admin_c_pins(ssc: FeatureCode) -> Result<CPINRefRange, Error> {
    match ssc {
        FeatureCode::Enterprise => Err(Error::IncompatibleSSC),
        FeatureCode::OpalV1 => Ok(spec::opal::locking::c_pin::ADMIN),
        FeatureCode::OpalV2 => Ok(spec::opal::locking::c_pin::ADMIN),
        FeatureCode::Opalite => Ok(ObjectUIDRange::new_count(spec::opalite::locking::c_pin::ADMIN1, 1, 1)),
        FeatureCode::PyriteV1 => Ok(spec::pyrite::locking::c_pin::ADMIN),
        FeatureCode::PyriteV2 => Ok(spec::pyrite::locking::c_pin::ADMIN),
        FeatureCode::Ruby => Ok(spec::ruby::locking::c_pin::ADMIN),
        FeatureCode::KeyPerIO => Ok(spec::kpio::key_per_io::c_pin::ADMIN),
        _ => Err(Error::IncompatibleSSC),
    }
}

pub fn get_general_lookup(ssc: FeatureCode) -> &'static dyn ObjectLookup {
    get_feature_lookup(ssc).unwrap_or(&spec::core::OBJECT_LOOKUP)
}

pub fn get_feature_lookup(ssc: FeatureCode) -> Option<&'static dyn ObjectLookup> {
    match ssc {
        FeatureCode::Enterprise => Some(&spec::enterprise::OBJECT_LOOKUP),
        FeatureCode::OpalV1 => Some(&spec::opal::OBJECT_LOOKUP),
        FeatureCode::OpalV2 => Some(&spec::opal::OBJECT_LOOKUP),
        FeatureCode::Opalite => Some(&spec::pyrite::OBJECT_LOOKUP),
        FeatureCode::PyriteV1 => Some(&spec::pyrite::OBJECT_LOOKUP),
        FeatureCode::PyriteV2 => Some(&spec::pyrite::OBJECT_LOOKUP),
        FeatureCode::Ruby => Some(&spec::ruby::OBJECT_LOOKUP),
        FeatureCode::KeyPerIO => Some(&spec::kpio::OBJECT_LOOKUP),
        FeatureCode::AdditionalDataStoreTables => Some(&spec::data_store::OBJECT_LOOKUP),
        _ => None,
    }
}

pub async fn start_admin1_session(tper: &TPer, admin1_password: &[u8]) -> Result<Session, Error> {
    let discovery = tper.discover().await?;
    let ssc = discovery.get_primary_ssc().ok_or(Error::IncompatibleSSC)?;
    let locking_sp = get_locking_sp(ssc.feature_code())?;
    let admin1 = get_locking_admins(ssc.feature_code())?.nth(1).unwrap();
    Ok(tper.start_session(locking_sp, Some(admin1), Some(admin1_password)).await?)
}
