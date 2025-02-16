use slint::{SharedString, VecModel};
use std::rc::Rc;

use crate::generated::ContentStatus;
use crate::generated::FeatureModel;
use crate::generated::SummaryModel;

impl SummaryModel {
    pub fn new(
        name: String,
        serial: String,
        path: String,
        firmware: String,
        interface: String,
        discovery_status: ContentStatus,
        discovery_error: String,
        security_subsystem_classes: Vec<String>,
        security_providers: Vec<String>,
        common_features: Vec<FeatureModel>,
        ssc_features: Vec<FeatureModel>,
    ) -> Self {
        let security_subsystem_classes: Vec<SharedString> =
            security_subsystem_classes.into_iter().map(|x| x.into()).collect();
        let security_providers: Vec<SharedString> = security_providers.into_iter().map(|x| x.into()).collect();
        Self {
            name: name.into(),
            serial: serial.into(),
            path: path.into(),
            firmware: firmware.into(),
            interface: interface.into(),
            discovery_status,
            discovery_error: discovery_error.into(),
            security_subsystem_classes: Rc::new(VecModel::from(security_subsystem_classes)).into(),
            security_providers: Rc::new(VecModel::from(security_providers)).into(),
            common_features: Rc::new(VecModel::from(common_features)).into(),
            ssc_features: Rc::new(VecModel::from(ssc_features)).into(),
        }
    }

    pub fn empty() -> Self {
        Self::new(
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            String::new(),
            ContentStatus::Loading,
            String::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
            Vec::new(),
        )
    }
}
