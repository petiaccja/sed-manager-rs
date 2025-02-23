use std::rc::Rc;

use sed_manager::{
    device::{Device, Error as DeviceError},
    tper::discover,
};
use slint::{Model, ModelRc, SharedString, ToSharedString, VecModel};

use crate::device_list::DeviceList;
use crate::ui;
use crate::utility::{run_in_thread, AtomicBorrow, Versioned};

pub struct AppState {
    pub window: ui::AppWindow,
    pub device_list: Versioned<Result<DeviceList, DeviceError>>,
    pub descriptions: ModelRc<ui::DeviceDescription>,
}

impl AppState {
    pub fn new(window: ui::AppWindow) -> Self {
        Self {
            window,
            device_list: Versioned::new(Ok(DeviceList::empty())),
            descriptions: ModelRc::new(VecModel::from(vec![])),
        }
    }
}

async fn update_device_list(app_state: Rc<AtomicBorrow<AppState>>) {
    let snapshot = app_state.with(|app_state| app_state.device_list.snapshot());
    let fresh = DeviceList::query().await;
    app_state.with_mut(|app_state| {
        snapshot.run_if_current(app_state.device_list.current(), || {
            app_state.device_list = Versioned::new(fresh);
        })
    });
}

struct SyncDeviceIdentity {
    name: String,
    serial: String,
    path: String,
    firmware: String,
    interface: String,
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
        device_snap.run_if_current(app_state.device_list.current(), || {
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
            app_state.descriptions = ModelRc::new(VecModel::from(descriptions));
            app_state.window.set_device_descriptions(app_state.descriptions.clone());
        });
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
                Ok(discovery) => {
                    let ui_discovery = ui::DeviceDiscovery::from_discovery(&discovery);
                    let Some(ui_desc) = app_state.descriptions.row_data(device_idx) else {
                        return;
                    };
                    let new_ui_desc = ui::DeviceDescription {
                        discovery: ui_discovery,
                        discovery_status: ui::ContentStatus::Success,
                        ..ui_desc
                    };
                    app_state.descriptions.set_row_data(device_idx, new_ui_desc);
                }
                Err(err) => {
                    let Some(ui_desc) = app_state.descriptions.row_data(device_idx) else {
                        return;
                    };
                    let new_ui_desc = ui::DeviceDescription {
                        discovery_status: ui::ContentStatus::Error,
                        discovery_error_message: err.to_shared_string(),
                        ..ui_desc
                    };
                    app_state.descriptions.set_row_data(device_idx, new_ui_desc);
                }
            };
        });
    });
}
