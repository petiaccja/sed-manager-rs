mod admin;
mod kpio;
mod locking;

pub use admin::Admin;
pub use kpio::Kpio;
pub use locking::Locking;

use sed_packet::discovery::{Discovery, Feature, FeatureDescriptor, SecuritySubsystemClass};
use sed_spec::{objects::SecurityProviderRef, preconfig::psid};

use crate::spec::admin::{AuthorityTable, CPinTable};

#[derive(Debug)]
pub struct Spec {
    /// The specification of the Admin SP of the TPer's SSC.
    pub admin: Admin,
    /// The specification of the Locking SP of the TPer's SSC.
    pub locking: Option<Locking>,
    /// The specification of the KPIO SP of the TPer's SSC.
    pub kpio: Option<Kpio>,
    pub discovery: Discovery,
}

impl Spec {
    pub fn default_ssc(&self) -> Option<&dyn SecuritySubsystemClass> {
        self.discovery.ssc_features().next()
    }

    pub fn secondary_sp_uid(&self) -> Option<SecurityProviderRef> {
        self.locking.as_ref().map(|sp| sp.uid).or_else(|| self.kpio.as_ref().map(|sp| sp.uid))
    }

    pub fn sort(discovery: &mut Discovery) {
        discovery.feature_descriptors.sort_by_key(feature_priority);
    }
}

impl TryFrom<Discovery> for Spec {
    type Error = Discovery;

    fn try_from(mut discovery: Discovery) -> Result<Self, Self::Error> {
        Self::sort(&mut discovery);
        let ssc = discovery.feature_descriptors.iter().find(|feature| feature.as_ssc().is_some());
        let (admin, locking, kpio) = match ssc {
            Some(feature) => ssc_spec(feature),
            None => (None, None, None),
        };
        match admin {
            Some(admin) => Ok(Self { admin, locking, kpio, discovery }),
            None => Err(discovery),
        }
    }
}

/// Assign a priority to each feature descriptor in the discovery.
///
/// This enables sorting the feature descriptors by priority. The main goal is
/// unambiguously pick the most capable SSC in case the device supports more
/// than one SSC.
///
/// The smaller the number, the higher the priority. The number can be used for
/// sorting directly.
fn feature_priority(feature: &FeatureDescriptor) -> usize {
    match feature {
        FeatureDescriptor::TPer(_) => 200,
        FeatureDescriptor::Locking(_) => 201,
        FeatureDescriptor::Geometry(_) => 202,
        FeatureDescriptor::DataRemoval(_) => 203,
        FeatureDescriptor::BlockSIDAuth(_) => 101,
        FeatureDescriptor::AdditionalDataStoreTables(_) => 100,
        FeatureDescriptor::Enterprise(_) => 7,
        FeatureDescriptor::OpalV1(_) => 2,
        FeatureDescriptor::OpalV2(_) => 0,
        FeatureDescriptor::Opalite(_) => 3,
        FeatureDescriptor::PyriteV1(_) => 5,
        FeatureDescriptor::PyriteV2(_) => 4,
        FeatureDescriptor::Ruby(_) => 1,
        FeatureDescriptor::KeyPerIO(_) => 6,
        FeatureDescriptor::Unrecognized(_, _) => 1000,
    }
}

fn ssc_spec(feature: &FeatureDescriptor) -> (Option<Admin>, Option<Locking>, Option<Kpio>) {
    match feature {
        FeatureDescriptor::Enterprise(_) => enterprise(),
        // The Opal 2 values are used for Opal 1 as well.
        FeatureDescriptor::OpalV1(_) => opal(),
        FeatureDescriptor::OpalV2(_) => opal(),
        FeatureDescriptor::Opalite(_) => opalite(),
        FeatureDescriptor::PyriteV1(_) => pyrite(),
        FeatureDescriptor::PyriteV2(_) => pyrite(),
        FeatureDescriptor::Ruby(_) => ruby(),
        FeatureDescriptor::KeyPerIO(_) => kpio(),
        _ => (None, None, None),
    }
}

fn enterprise() -> (Option<Admin>, Option<Locking>, Option<Kpio>) {
    use sed_spec::preconfig::enterprise::admin;
    (
        Some(Admin {
            uid: admin::sp::ADMIN,
            authorities: AuthorityTable { sid: admin::authority::SID, psid: psid::admin::authority::PSID },
            c_pins: CPinTable { sid: admin::c_pin::SID, msid: admin::c_pin::MSID },
        }),
        Some(Locking { uid: admin::sp::LOCKING }),
        None,
    )
}

fn opal() -> (Option<Admin>, Option<Locking>, Option<Kpio>) {
    use sed_spec::preconfig::opal_2::admin;
    (
        Some(Admin {
            uid: admin::sp::ADMIN,
            authorities: AuthorityTable { sid: admin::authority::SID, psid: psid::admin::authority::PSID },
            c_pins: CPinTable { sid: admin::c_pin::SID, msid: admin::c_pin::MSID },
        }),
        Some(Locking { uid: admin::sp::LOCKING }),
        None,
    )
}

fn opalite() -> (Option<Admin>, Option<Locking>, Option<Kpio>) {
    use sed_spec::preconfig::opalite::admin;
    (
        Some(Admin {
            uid: admin::sp::ADMIN,
            authorities: AuthorityTable { sid: admin::authority::SID, psid: psid::admin::authority::PSID },
            c_pins: CPinTable { sid: admin::c_pin::SID, msid: admin::c_pin::MSID },
        }),
        Some(Locking { uid: admin::sp::LOCKING }),
        None,
    )
}

fn pyrite() -> (Option<Admin>, Option<Locking>, Option<Kpio>) {
    use sed_spec::preconfig::pyrite_2::admin;
    (
        Some(Admin {
            uid: admin::sp::ADMIN,
            authorities: AuthorityTable { sid: admin::authority::SID, psid: psid::admin::authority::PSID },
            c_pins: CPinTable { sid: admin::c_pin::SID, msid: admin::c_pin::MSID },
        }),
        Some(Locking { uid: admin::sp::LOCKING }),
        None,
    )
}

fn ruby() -> (Option<Admin>, Option<Locking>, Option<Kpio>) {
    use sed_spec::preconfig::ruby::admin;
    (
        Some(Admin {
            uid: admin::sp::ADMIN,
            authorities: AuthorityTable { sid: admin::authority::SID, psid: psid::admin::authority::PSID },
            c_pins: CPinTable { sid: admin::c_pin::SID, msid: admin::c_pin::MSID },
        }),
        Some(Locking { uid: admin::sp::LOCKING }),
        None,
    )
}

fn kpio() -> (Option<Admin>, Option<Locking>, Option<Kpio>) {
    use sed_spec::preconfig::kpio::admin;
    (
        Some(Admin {
            uid: admin::sp::ADMIN,
            authorities: AuthorityTable { sid: admin::authority::SID, psid: psid::admin::authority::PSID },
            c_pins: CPinTable { sid: admin::c_pin::SID, msid: admin::c_pin::MSID },
        }),
        None,
        Some(Kpio { uid: admin::sp::KEY_PER_IO }),
    )
}
