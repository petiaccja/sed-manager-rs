use sed_manager::messaging::discovery::{Discovery, FeatureCode};
use slint::{SharedString, ToSharedString as _, VecModel};
use std::rc::Rc;

use crate::ui::{ContentStatus, DeviceDescription, DeviceDiscovery, DeviceDiscoveryFeature, DeviceIdentity};

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
        discovery_status: ContentStatus,
        discovery_error_message: String,
        discovery: DeviceDiscovery,
    ) -> Self {
        Self {
            discovery: discovery,
            discovery_error_message: discovery_error_message.into(),
            discovery_status: discovery_status,
            identity: identity,
        }
    }

    pub fn empty() -> Self {
        Self::new(DeviceIdentity::empty(), ContentStatus::Loading, String::new(), DeviceDiscovery::empty())
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
