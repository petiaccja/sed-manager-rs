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

pub fn get_lookup(ssc: FeatureCode) -> &'static dyn ObjectLookup {
    match ssc {
        FeatureCode::Enterprise => &spec::enterprise::OBJECT_LOOKUP,
        FeatureCode::OpalV1 => &spec::opal::OBJECT_LOOKUP,
        FeatureCode::OpalV2 => &spec::opal::OBJECT_LOOKUP,
        FeatureCode::Opalite => &spec::pyrite::OBJECT_LOOKUP,
        FeatureCode::PyriteV1 => &spec::pyrite::OBJECT_LOOKUP,
        FeatureCode::PyriteV2 => &spec::pyrite::OBJECT_LOOKUP,
        FeatureCode::Ruby => &spec::ruby::OBJECT_LOOKUP,
        FeatureCode::KeyPerIO => &spec::kpio::OBJECT_LOOKUP,
        FeatureCode::AdditionalDataStoreTables => &spec::data_store::OBJECT_LOOKUP,
        _ => &spec::core::OBJECT_LOOKUP,
    }
}

pub async fn start_admin1_session(tper: &TPer, admin1_password: &[u8]) -> Result<Session, Error> {
    let discovery = tper.discover().await?;
    let ssc = discovery.get_primary_ssc().ok_or(Error::IncompatibleSSC)?;
    let locking_sp = get_locking_sp(ssc.feature_code())?;
    let admin1 = get_locking_admins(ssc.feature_code())?.nth(1).unwrap();
    Ok(tper.start_session(locking_sp, Some(admin1), Some(admin1_password)).await?)
}

#[cfg(test)]
pub mod tests {
    use std::sync::Arc;

    use crate::fake_device::FakeDevice;
    use crate::rpc::TokioRuntime;
    use crate::spec;
    use crate::tper::TPer;

    pub async fn setup_activated_tper() -> TPer {
        use spec::opal::admin::sp::LOCKING;
        let runtime = Arc::new(TokioRuntime::new());
        let device = Arc::new(FakeDevice::new());
        device.controller().lock().unwrap().activate(LOCKING).unwrap();
        TPer::new_on_default_com_id(device, runtime).unwrap()
    }
}
