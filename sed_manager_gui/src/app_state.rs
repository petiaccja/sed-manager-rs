use std::sync::Arc;

use sed_manager::tper::Session;
use sed_manager::{device::Error as DeviceError, messaging::discovery::Discovery, rpc::Error as RPCError, tper::TPer};
use slint::{Model, ModelRc, SharedString, ToSharedString, VecModel};

use crate::ui;
use crate::utility::Versioned;
use crate::{device_list::DeviceList, ui::ActionResult};

pub struct AppState {
    window: ui::AppWindow,
    device_list: Versioned<Result<DeviceList, DeviceError>>,
    discoveries: Vec<Option<Discovery>>,
    tpers: Vec<Option<Arc<TPer>>>,
    sessions: Vec<Option<Arc<Session>>>,
    descriptions: ModelRc<ui::DeviceDescription>,
    action_results: ModelRc<ui::ActionResult>,
    locking_range_errors: ModelRc<ModelRc<SharedString>>,
    locking_ranges: ModelRc<ModelRc<ui::LockingRange>>,
}

pub struct SyncDeviceIdentity {
    pub name: String,
    pub serial: String,
    pub path: String,
    pub firmware: String,
    pub interface: String,
}

impl AppState {
    pub fn new(window: ui::AppWindow) -> Self {
        Self {
            window,
            device_list: Versioned::new(Ok(DeviceList::empty())),
            discoveries: vec![],
            tpers: vec![],
            sessions: vec![],
            descriptions: ModelRc::new(VecModel::from(vec![])),
            action_results: ModelRc::new(VecModel::from(vec![])),
            locking_range_errors: ModelRc::new(VecModel::from(vec![])),
            locking_ranges: ModelRc::new(VecModel::from(vec![])),
        }
    }

    pub fn set_device_list(&mut self, device_list: DeviceList) {
        self.device_list = Versioned::new(Ok(device_list));
        self.init_from_devices();
    }

    pub fn get_device_list(&self) -> &Versioned<Result<DeviceList, DeviceError>> {
        &self.device_list
    }

    pub fn set_device_list_error(&mut self, error: DeviceError) {
        self.device_list = Versioned::new(Err(error));
        self.init_from_devices();
    }

    pub fn init_from_devices(&mut self) {
        let num_devices = self.device_list.as_ref().map(|dev| dev.devices.len()).unwrap_or(0);
        let discoveries = core::iter::repeat_with(|| None).take(num_devices).collect::<Vec<_>>();
        let tpers = core::iter::repeat_with(|| None).take(num_devices).collect::<Vec<_>>();
        let sessions = core::iter::repeat_with(|| None).take(num_devices).collect::<Vec<_>>();
        let unavailable_devices: Vec<_> = self
            .device_list
            .as_ref()
            .map(|dev_list| &dev_list.unavailable_devices)
            .unwrap_or(&vec![])
            .iter()
            .map(|(path, error)| ui::UnavailableDevice::new(path.clone(), error.to_string()))
            .collect();
        let action_results = core::iter::repeat_with(|| ActionResult::success()).take(num_devices).collect::<Vec<_>>();
        let locking_range_error_logs = core::iter::repeat_with(|| ModelRc::new(VecModel::from(vec![])))
            .take(num_devices)
            .collect::<Vec<_>>();
        let locking_ranges = core::iter::repeat_with(|| ModelRc::new(VecModel::from(vec![])))
            .take(num_devices)
            .collect::<Vec<_>>();

        self.discoveries = discoveries;
        self.tpers = tpers;
        self.sessions = sessions;
        self.action_results = ModelRc::new(VecModel::from(action_results));
        self.locking_range_errors = ModelRc::new(VecModel::from(locking_range_error_logs));
        self.locking_ranges = ModelRc::new(VecModel::from(locking_ranges));
        self.window.set_unavailable_devices(ModelRc::new(VecModel::from(unavailable_devices)));
        self.window.set_action_results(self.action_results.clone());
        self.window.set_locking_range_error_logs(self.locking_range_errors.clone());
        self.window.set_locking_ranges(self.locking_ranges.clone());
    }

    pub fn set_device_identities(&mut self, identities: Vec<SyncDeviceIdentity>) {
        let descriptions = identities
            .into_iter()
            .map(|id| {
                ui::DeviceDescription::new(
                    ui::DeviceIdentity::new(id.name, id.serial, id.path, id.firmware, id.interface),
                    ui::ContentStatus::Loading,
                    String::new(),
                    ui::DeviceDiscovery::empty(),
                )
            })
            .collect::<Vec<_>>();
        self.descriptions = ModelRc::new(VecModel::from(descriptions));
        self.window.set_device_descriptions(self.descriptions.clone());
    }

    pub fn set_tabs(&mut self, device_names: Vec<String>) {
        let mut tabs = device_names.into_iter().map(|name| SharedString::from(name)).collect::<Vec<_>>();
        if self.device_list.as_ref().is_ok_and(|dl| !dl.unavailable_devices.is_empty()) {
            tabs.push("Unavailable devices".into());
        }
        self.window.set_device_tabs(ModelRc::new(VecModel::from(tabs)));
    }

    pub fn get_discovery(&self, device_idx: usize) -> Option<&Discovery> {
        self.discoveries.get(device_idx).and_then(|x| x.as_ref())
    }

    pub fn set_discovery(&mut self, device_idx: usize, discovery: Discovery) {
        let ui_discovery = ui::DeviceDiscovery::from_discovery(&discovery);
        let Some(ui_desc) = self.descriptions.row_data(device_idx) else {
            return;
        };
        let new_ui_desc =
            ui::DeviceDescription { discovery: ui_discovery, discovery_status: ui::ContentStatus::Success, ..ui_desc };
        self.descriptions.set_row_data(device_idx, new_ui_desc);
        self.discoveries.get_mut(device_idx).map(|inner| *inner = Some(discovery));
    }

    pub fn set_discovery_error(&mut self, device_idx: usize, error: RPCError) {
        let Some(ui_desc) = self.descriptions.row_data(device_idx) else {
            return;
        };
        let new_ui_desc = ui::DeviceDescription {
            discovery_status: ui::ContentStatus::Error,
            discovery_error_message: error.to_shared_string(),
            ..ui_desc
        };
        self.descriptions.set_row_data(device_idx, new_ui_desc);
        self.discoveries.get_mut(device_idx).map(|inner| *inner = None);
    }

    pub fn get_tper(&mut self, device_idx: usize) -> Option<Arc<TPer>> {
        let tper_ref = self.tpers.get_mut(device_idx)?;
        if let Some(tper) = tper_ref {
            return Some(tper.clone());
        }
        let device = self.device_list.as_ref().ok().and_then(|x| x.devices.get(device_idx))?;
        let discovery = self.discoveries.get(device_idx).and_then(|x| x.as_ref())?;
        let ssc = discovery.get_primary_ssc()?;
        let tper = Arc::new(TPer::new(device.arc(), ssc.base_com_id(), 0));
        *tper_ref = Some(tper.clone());
        Some(tper)
    }

    pub fn get_session(&self, device_idx: usize) -> Option<Arc<Session>> {
        self.sessions.get(device_idx).cloned().flatten()
    }

    pub fn take_session(&mut self, device_idx: usize) -> Option<Arc<Session>> {
        self.sessions.get_mut(device_idx).map(|s| s.take()).flatten()
    }

    pub fn set_session(&mut self, device_idx: usize, session: Arc<Session>) -> Option<Arc<Session>> {
        self.sessions.get_mut(device_idx).map(|s| s.replace(session)).flatten()
    }

    pub fn set_action_result(&mut self, device_idx: usize, action_result: ActionResult) {
        let action_results = &self.action_results;
        if device_idx < action_results.row_count() {
            action_results.set_row_data(device_idx, action_result);
        }
    }

    pub fn append_locking_range_error(&mut self, device_idx: usize, error: String) {
        let Some(device_logs) = self.locking_range_errors.row_data(device_idx) else {
            return;
        };
        let device_logs = device_logs.as_any().downcast_ref::<VecModel<SharedString>>().unwrap();
        device_logs.push(error.into());
    }

    pub fn clear_locking_range_errors(&mut self, device_idx: usize) {
        let Some(device_logs) = self.locking_range_errors.row_data(device_idx) else {
            return;
        };
        let device_logs = device_logs.as_any().downcast_ref::<VecModel<SharedString>>().unwrap();
        device_logs.clear();
    }

    pub fn append_locking_range(&mut self, device_idx: usize, locking_range: ui::LockingRange) {
        let Some(device_ranges) = self.locking_ranges.row_data(device_idx) else {
            return;
        };
        let device_logs = device_ranges.as_any().downcast_ref::<VecModel<ui::LockingRange>>().unwrap();
        device_logs.push(locking_range);
    }

    pub fn clear_locking_ranges(&mut self, device_idx: usize) {
        let Some(device_ranges) = self.locking_ranges.row_data(device_idx) else {
            return;
        };
        let device_logs = device_ranges.as_any().downcast_ref::<VecModel<ui::LockingRange>>().unwrap();
        device_logs.clear();
    }
}
