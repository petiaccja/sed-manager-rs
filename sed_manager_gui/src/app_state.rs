use std::sync::Arc;

use sed_manager::device::{Device, Error as DeviceError};
use slint::{ModelRc, VecModel};

use crate::generated::{AppWindow, SummaryModel};

pub struct AppState {
    pub window: AppWindow,
    pub device_list: Arc<DeviceList>,
    pub summaries: ModelRc<SummaryModel>,
}

pub struct DeviceList {
    pub active_devices: Vec<Arc<dyn Device>>,
    pub failed_devices: Vec<(String, DeviceError)>,
    pub failed_enumeration: Option<DeviceError>,
}

impl AppState {
    pub fn new(window: AppWindow) -> Self {
        Self { window, device_list: Arc::new(DeviceList::empty()), summaries: ModelRc::new(VecModel::from(vec![])) }
    }
}

impl DeviceList {
    pub fn new() -> Self {
        Self { active_devices: vec![], failed_devices: vec![], failed_enumeration: None }
    }

    pub fn empty() -> Self {
        Self::new()
    }

    pub fn from_failed_enumeration(error: DeviceError) -> Self {
        Self { active_devices: vec![], failed_devices: vec![], failed_enumeration: Some(error) }
    }

    pub fn from_devices(active_devices: Vec<Box<dyn Device>>, failed_devices: Vec<(String, DeviceError)>) -> Self {
        let active_devices = active_devices.into_iter().map(|x| x.into()).collect();
        Self { active_devices, failed_devices, failed_enumeration: None }
    }
}
