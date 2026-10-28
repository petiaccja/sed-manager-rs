//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_packet::discovery::{
    AdditionalDataStoreTablesDescriptor, BlockSIDAuthDescriptor, DataRemovalDescriptor, Discovery, FeatureDescriptor,
    GeometryDescriptor, SscFeatureDescriptor,
};
use sed_packet::discovery::{LockingDescriptor, TperDescriptor};
use sed_spec::objects::SecurityProviderRef;
use sed_spec::preconfig::{enterprise, opalite, pyrite_2, ruby};
use sed_spec::preconfig::{kpio, opal_2};

use crate::Error;

/// The specifications a self-encrypting drive implements.
#[derive(Debug, Clone)]
pub struct DeviceSpec {
    pub tper_desc: TperDescriptor,
    /// The most capable SSC of the device. For example, if the device supports
    /// both Opal 2.0 and Opalite, Opal 2.0 will be selected as its essentially
    /// a superset of Opalite.
    pub ssc_desc: SscFeatureDescriptor,
    pub locking_desc: Option<LockingDescriptor>,
    pub geometry_desc: Option<GeometryDescriptor>,
    pub data_removal_desc: Option<DataRemovalDescriptor>,
    pub block_sid_auth_desc: Option<BlockSIDAuthDescriptor>,
    pub additional_data_stores_desc: Option<AdditionalDataStoreTablesDescriptor>,
    /// SSCs that the device supports in addition to its most capable SSC.
    pub additional_ssc_descs: Vec<SscFeatureDescriptor>,
}

impl DeviceSpec {
    /// Determine the specifications a device implements from its L0 discovery.
    ///
    /// # Errors
    ///
    /// If the drive is not a TCG SED, or if it supports no SSCs, an error is
    /// returned.
    pub fn try_new(discovery: Discovery) -> Result<Self, Error> {
        // Take SSC descriptors from discovery.
        let (mut ssc_descs, generic_descs): (Vec<_>, Vec<_>) =
            discovery.into_iter().partition(FeatureDescriptor::is_ssc);

        // Find the most capable SSC.
        sort_by_priority(&mut ssc_descs);
        ssc_descs.reverse();
        let mut ssc_descs: Vec<_> = ssc_descs
            .into_iter()
            .map(|ssc_desc| ssc_desc.to_ssc().expect("the partitioning must have kept only SSC descs"))
            .collect();
        let ssc_desc = ssc_descs.pop().ok_or(Error::NoSscAvailable)?;

        // Get additional important descriptors.
        let generic_discovery = Discovery { feature_descriptors: generic_descs, ..Default::default() };
        let tper_desc = generic_discovery.get::<TperDescriptor>().ok_or(Error::NotTcgDevice)?.clone();
        let locking_desc = generic_discovery.get().cloned();
        let geometry_desc = generic_discovery.get().cloned();
        let data_removal_desc = generic_discovery.get().cloned();
        let block_sid_auth_desc = generic_discovery.get().cloned();
        let additional_data_stores_desc = generic_discovery.get().cloned();

        Ok(Self {
            tper_desc,
            locking_desc,
            ssc_desc,
            additional_ssc_descs: ssc_descs,
            geometry_desc,
            data_removal_desc,
            block_sid_auth_desc,
            additional_data_stores_desc,
        })
    }

    /// Return the UID of the Admin SP.
    pub fn admin_sp(&self) -> SecurityProviderRef {
        // The admin SP is the same for all SSCs.
        opal_2::admin::sp::ADMIN
    }

    /// Return the UID of the secondary SP: this is either the Locking SP or the
    /// KPIO SC, depending on the SSC.
    pub fn secondary_sp(&self) -> SecurityProviderRef {
        use SscFeatureDescriptor::*;
        match &self.ssc_desc {
            KeyPerIO(_) => kpio::admin::sp::KEY_PER_IO,
            Enterprise(_) => enterprise::admin::sp::LOCKING,
            OpalV1(_) => opal_2::admin::sp::LOCKING,
            OpalV2(_) => opal_2::admin::sp::LOCKING,
            Opalite(_) => opalite::admin::sp::LOCKING,
            PyriteV1(_) => pyrite_2::admin::sp::LOCKING,
            PyriteV2(_) => pyrite_2::admin::sp::LOCKING,
            Ruby(_) => ruby::admin::sp::LOCKING,
        }
    }
}

/// Sort the feature descriptors by their priority. See [feature_priority].
pub fn sort_by_priority(feature_descriptors: &mut [FeatureDescriptor]) {
    feature_descriptors.sort_by_key(feature_priority);
}

/// Assign a priority to each feature descriptor in the discovery.
///
/// This enables sorting the feature descriptors by priority. The main goal is
/// unambiguously pick the most capable SSC in case the device supports more
/// than one SSC.
///
/// The smaller the number, the higher the priority. The number can be used for
/// sorting directly.
pub fn feature_priority(feature: &FeatureDescriptor) -> usize {
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
