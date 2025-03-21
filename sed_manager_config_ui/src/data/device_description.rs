//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use sed_manager::messaging::discovery::{Discovery, FeatureCode, GeometryDescriptor};
use slint::{SharedString, ToSharedString as _, VecModel};
use std::rc::Rc;

use crate::{
    ActivitySupport, DeviceDescription, DeviceDiscovery, DeviceDiscoveryFeature, DeviceGeometry, DeviceIdentity,
    ExtendedStatus,
};

impl DeviceIdentity {
    pub fn new(name: String, serial: String, path: String, firmware: String, interface: String) -> Self {
        Self {
            name: name.into(),
            serial: serial.into(),
            path: path.into(),
            firmware: firmware.into(),
            interface: interface.into(),
        }
    }

    pub fn empty() -> Self {
        Self::new(String::new(), String::new(), String::new(), String::new(), String::new())
    }
}

impl DeviceDiscovery {
    pub fn new(
        security_subsystem_classes: Vec<String>,
        security_providers: Vec<String>,
        common_features: Vec<DeviceDiscoveryFeature>,
        ssc_features: Vec<DeviceDiscoveryFeature>,
    ) -> Self {
        let security_subsystem_classes: Vec<SharedString> =
            security_subsystem_classes.into_iter().map(|x| x.into()).collect();
        let security_providers: Vec<SharedString> = security_providers.into_iter().map(|x| x.into()).collect();
        Self {
            security_subsystem_classes: Rc::new(VecModel::from(security_subsystem_classes)).into(),
            security_providers: Rc::new(VecModel::from(security_providers)).into(),
            common_features: Rc::new(VecModel::from(common_features)).into(),
            ssc_features: Rc::new(VecModel::from(ssc_features)).into(),
        }
    }

    pub fn empty() -> Self {
        Self::new(Vec::new(), Vec::new(), Vec::new(), Vec::new())
    }

    pub fn from_discovery(discovery: &Discovery) -> Self {
        let common_features: Vec<_> =
            discovery.get_common_features().map(|desc| DeviceDiscoveryFeature::from(desc)).collect();
        let ssc_features: Vec<_> = discovery
            .iter()
            .filter(|desc| desc.security_subsystem_class().is_some())
            .map(|desc| DeviceDiscoveryFeature::from(desc))
            .collect();
        let ssc: Vec<_> = discovery.get_ssc_features().map(|desc| desc.feature_code()).collect();
        let sp = ssc.first().map(|ssc| get_security_providers(&ssc)).unwrap_or(vec![]);

        let sp = sp.into_iter().map(|x| x.into()).collect::<Vec<_>>();
        let ssc = ssc.into_iter().map(|x| x.to_shared_string()).collect::<Vec<_>>();

        Self {
            security_subsystem_classes: Rc::new(VecModel::from(ssc)).into(),
            security_providers: Rc::new(VecModel::from(sp)).into(),
            common_features: Rc::new(VecModel::from(common_features)).into(),
            ssc_features: Rc::new(VecModel::from(ssc_features)).into(),
        }
    }
}

impl DeviceDescription {
    pub fn new(
        identity: DeviceIdentity,
        discovery_status: ExtendedStatus,
        discovery: DeviceDiscovery,
        activity_support: ActivitySupport,
        geometry: DeviceGeometry,
    ) -> Self {
        Self { discovery, discovery_status, identity, activity_support, geometry }
    }

    pub fn empty() -> Self {
        Self::new(
            DeviceIdentity::empty(),
            ExtendedStatus::error("".into()),
            DeviceDiscovery::empty(),
            ActivitySupport::none(),
            DeviceGeometry::unknown(),
        )
    }
}

impl DeviceGeometry {
    pub fn new(block_size: u32, block_alignment: u64, lowest_aligned_block: u64) -> Self {
        Self {
            block_size: block_size as i32,
            block_alignment: block_alignment as i32,
            lowest_aligned_block: lowest_aligned_block as i32,
        }
    }

    pub fn unknown() -> Self {
        Self::new(0, 1, 0)
    }

    pub fn from_discovery(discovery: &Discovery) -> Self {
        if let Some(geometry) = discovery.get::<GeometryDescriptor>() {
            if geometry.align {
                Self::new(geometry.logical_block_size, geometry.alignment_granularity, geometry.lowest_aligned_lba)
            } else {
                Self::new(geometry.logical_block_size, 1, 0)
            }
        } else {
            Self::unknown()
        }
    }
}

fn get_security_providers(ssc: &FeatureCode) -> Vec<&str> {
    match ssc {
        FeatureCode::Enterprise => vec!["Admin", "Locking"],
        FeatureCode::OpalV1 => vec!["Admin", "Locking"],
        FeatureCode::OpalV2 => vec!["Admin", "Locking"],
        FeatureCode::Opalite => vec!["Admin", "Locking"],
        FeatureCode::PyriteV1 => vec!["Admin", "Locking"],
        FeatureCode::PyriteV2 => vec!["Admin", "Locking"],
        FeatureCode::Ruby => vec!["Admin", "Locking"],
        FeatureCode::KeyPerIO => vec!["Admin", "KeyPerIO"],
        _ => vec![],
    }
}
