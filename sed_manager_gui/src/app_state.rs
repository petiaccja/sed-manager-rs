use std::{rc::Rc, sync::Arc};

use sed_manager::{
    applications::{self, get_default_ssc},
    device::{Device, Error as DeviceError},
    messaging::discovery::Discovery,
    rpc::Error as RPCError,
    tper::{discover, TPer},
};
use slint::{Model, ModelRc, SharedString, ToSharedString, VecModel};

use crate::ui;
use crate::utility::{run_in_thread, AtomicBorrow, Versioned};
use crate::{device_list::DeviceList, ui::ActionResult};

pub struct AppState {
    pub window: ui::AppWindow,
    pub device_list: Versioned<Result<DeviceList, DeviceError>>,
    pub discoveries: Vec<Option<Discovery>>,
    pub tpers: Vec<Option<Arc<TPer>>>,
    pub descriptions: ModelRc<ui::DeviceDescription>,
    pub action_results: ModelRc<ui::ActionResult>,
}

struct SyncDeviceIdentity {
    name: String,
    serial: String,
    path: String,
    firmware: String,
    interface: String,
}

impl AppState {
    pub fn new(window: ui::AppWindow) -> Self {
        Self {
            window,
            device_list: Versioned::new(Ok(DeviceList::empty())),
            discoveries: vec![],
            tpers: vec![],
            descriptions: ModelRc::new(VecModel::from(vec![])),
            action_results: ModelRc::new(VecModel::from(vec![])),
        }
    }

    fn set_device_list(&mut self, device_list: DeviceList) {
        self.device_list = Versioned::new(Ok(device_list));
        self.init_from_devices();
    }

    fn set_device_list_error(&mut self, error: DeviceError) {
        self.device_list = Versioned::new(Err(error));
        self.init_from_devices();
    }

    fn init_from_devices(&mut self) {
        let num_devices = self.device_list.as_ref().map(|dev| dev.devices.len()).unwrap_or(0);
        let action_results = std::iter::repeat_with(|| ActionResult::loading()).take(num_devices).collect::<Vec<_>>();
        let discoveries = std::iter::repeat_with(|| None).take(num_devices).collect::<Vec<_>>();
        let tpers = std::iter::repeat_with(|| None).take(num_devices).collect::<Vec<_>>();

        self.discoveries = discoveries;
        self.tpers = tpers;
        self.action_results = ModelRc::new(VecModel::from(action_results));
        self.window.set_action_results(self.action_results.clone());
    }

    fn set_device_identities(&mut self, identities: Vec<SyncDeviceIdentity>) {
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

    fn set_discovery(&mut self, device_idx: usize, discovery: Discovery) {
        let ui_discovery = ui::DeviceDiscovery::from_discovery(&discovery);
        let Some(ui_desc) = self.descriptions.row_data(device_idx) else {
            return;
        };
        let new_ui_desc =
            ui::DeviceDescription { discovery: ui_discovery, discovery_status: ui::ContentStatus::Success, ..ui_desc };
        self.descriptions.set_row_data(device_idx, new_ui_desc);
        self.discoveries.get_mut(device_idx).map(|inner| *inner = Some(discovery));
    }

    fn set_discovery_error(&mut self, device_idx: usize, error: RPCError) {
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

    fn get_tper(&mut self, device_idx: usize) -> Option<Arc<TPer>> {
        let tper_ref = self.tpers.get_mut(device_idx)?;
        if let Some(tper) = tper_ref {
            return Some(tper.clone());
        }
        let device = self.device_list.as_ref().ok().and_then(|x| x.devices.get(device_idx))?;
        let discovery = self.discoveries.get(device_idx).and_then(|x| x.as_ref())?;
        let default_ssc = get_default_ssc(discovery).ok()?;
        let ssc = default_ssc.security_subsystem_class()?;
        let tper = Arc::new(TPer::new(device.arc(), ssc.base_com_id(), 0));
        *tper_ref = Some(tper.clone());
        Some(tper)
    }

    fn set_action_result(&mut self, device_idx: usize, action_result: ActionResult) {
        let action_results = &self.action_results;
        if device_idx < action_results.row_count() {
            action_results.set_row_data(device_idx, action_result);
        }
    }
}

async fn update_device_list(app_state: Rc<AtomicBorrow<AppState>>) {
    let snapshot = app_state.with(|app_state| app_state.device_list.snapshot());
    let fresh = DeviceList::query().await;
    app_state.with_mut(|app_state| {
        snapshot.run_if_current(app_state.device_list.current(), || match fresh {
            Ok(value) => app_state.set_device_list(value),
            Err(error) => app_state.set_device_list_error(error),
        })
    });
}

async fn update_device_descriptions(app_state: Rc<AtomicBorrow<AppState>>) {
    let (devices, device_snap) =
        app_state.with(|app_state| (app_state.device_list.arc(), app_state.device_list.snapshot()));

    let identities = run_in_thread(move || match &*devices {
        Ok(devices) => devices
            .devices
            .iter()
            .map(|device| SyncDeviceIdentity {
                name: device.model_number().unwrap_or("Unknown model".into()),
                serial: device.serial_number().unwrap_or("Unknown serial".into()),
                path: device.path().unwrap_or("Unknown path".into()),
                firmware: device.firmware_revision().unwrap_or("Unknown firmware".into()),
                interface: device.interface().map(|x| x.to_string()).unwrap_or("Unknown interface".into()),
            })
            .collect::<Vec<_>>(),
        Err(_err) => vec![],
    })
    .await;

    app_state.with_mut(|app_state| {
        device_snap.run_if_current(app_state.device_list.current(), || app_state.set_device_identities(identities));
    });
}

async fn update_unavailable_devices(app_state: Rc<AtomicBorrow<AppState>>) {
    app_state.with(|app_state| {
        if let Ok(device_list) = &*app_state.device_list {
            let unavailable_devices: Vec<_> = device_list
                .unavailable_devices
                .iter()
                .map(|(path, error)| ui::UnavailableDevice::new(path.clone(), error.to_string()))
                .collect();
            app_state.window.set_unavailable_devices(ModelRc::new(VecModel::from(unavailable_devices)));
        }
    });
}

async fn update_device_tabs(app_state: Rc<AtomicBorrow<AppState>>) {
    let (devices, device_snap) =
        app_state.with(|app_state| (app_state.device_list.arc(), app_state.device_list.snapshot()));

    let names = run_in_thread(move || match &*devices {
        Ok(devices) => devices
            .devices
            .iter()
            .map(|device| device.model_number().unwrap_or("Unknown model".into()))
            .collect::<Vec<_>>(),
        Err(_err) => vec![],
    })
    .await;

    app_state.with_mut(|app_state| {
        device_snap.run_if_current(app_state.device_list.current(), || {
            let mut tabs = names.into_iter().map(|name| SharedString::from(name)).collect::<Vec<_>>();
            if app_state.device_list.as_ref().is_ok_and(|dl| !dl.unavailable_devices.is_empty()) {
                tabs.push("Unavailable devices".into());
            }
            app_state.window.set_device_tabs(ModelRc::new(VecModel::from(tabs)));
        });
    });
}

pub async fn update_devices(app_state: Rc<AtomicBorrow<AppState>>) {
    update_device_list(app_state.clone()).await;
    update_device_descriptions(app_state.clone()).await;
    update_unavailable_devices(app_state.clone()).await;
    update_device_tabs(app_state.clone()).await;
    let num_devs =
        app_state.with(|app_state| app_state.device_list.as_ref().map(|devs| devs.devices.len()).unwrap_or(0));
    for dev_idx in 0..num_devs {
        update_device_discovery(app_state.clone(), dev_idx).await;
    }
}

pub async fn update_device_discovery(app_state: Rc<AtomicBorrow<AppState>>, device_idx: usize) {
    fn get_device(app_state: &AppState, device_idx: usize) -> Option<&Versioned<dyn Device>> {
        app_state.device_list.as_ref().ok().map(|devs| devs.devices.get(device_idx)).flatten()
    }

    let Some((device, snap)) =
        app_state.with(|app_state| get_device(app_state, device_idx).map(|dev| (dev.arc(), dev.snapshot())))
    else {
        return;
    };

    let discovery = run_in_thread(move || discover(&*device)).await;
    app_state.with_mut(move |app_state| {
        let Some(device) = get_device(app_state, device_idx) else {
            return;
        };
        snap.run_if_current(device.current(), move || {
            match discovery {
                Ok(discovery) => app_state.set_discovery(device_idx, discovery),
                Err(error) => app_state.set_discovery_error(device_idx, error),
            };
        });
    });
}

pub async fn take_ownership(app_state: Rc<AtomicBorrow<AppState>>, device_idx: usize, new_password: String) {
    let tper = app_state.with_mut(|app_state| app_state.get_tper(device_idx));

    match tper {
        Some(tper) => {
            let result = applications::take_ownership(&*tper, new_password.as_bytes()).await;
            let action_result = match result {
                Ok(_) => ActionResult::success(),
                Err(err) => ActionResult::error(err.to_string()),
            };
            app_state.with_mut(|app_state| {
                app_state.set_action_result(device_idx, action_result);
            });
        }
        None => {
            app_state.with_mut(|app_state| {
                let action_result = ActionResult::error("could not open device to configure encryption".into());
                app_state.set_action_result(device_idx, action_result);
            });
        }
    }
}

pub async fn activate_locking(
    app_state: Rc<AtomicBorrow<AppState>>,
    device_idx: usize,
    sid_password: String,
    new_admin1_password: Option<String>,
) {
    let tper = app_state.with_mut(|app_state| app_state.get_tper(device_idx));

    match tper {
        Some(tper) => {
            let result = applications::activate_locking(
                &*tper,
                sid_password.as_bytes(),
                new_admin1_password.as_ref().map(|s| s.as_bytes()),
            )
            .await;
            let action_result = match result {
                Ok(_) => ActionResult::success(),
                Err(err) => ActionResult::error(err.to_string()),
            };
            app_state.with_mut(|app_state| {
                app_state.set_action_result(device_idx, action_result);
            });
        }
        None => {
            app_state.with_mut(|app_state| {
                let action_result = ActionResult::error("could not open device to configure encryption".into());
                app_state.set_action_result(device_idx, action_result);
            });
        }
    }
}
