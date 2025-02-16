use slint::{ModelRc, VecModel};
use std::rc::Rc;
use std::sync::Arc;

use sed_manager::device::{list_physical_drives, open_device, Interface};
use sed_manager::fake_device::FakeDevice;

use crate::app_state::{AppState, DeviceList};
use crate::atomic_borrow::AtomicBorrow;
use crate::generated::{AdditionalDrivesModel, ContentStatus, SummaryModel};
use crate::utility::run_in_thread;

fn get_device_list_blocking() -> DeviceList {
    let device_paths = match list_physical_drives() {
        Ok(device_paths) => device_paths,
        Err(error) => return DeviceList::from_failed_enumeration(error),
    };
    let maybe_devices: Vec<_> = device_paths
        .into_iter()
        .map(move |path| open_device(&path).map_err(|error| (path, error)))
        .collect();
    let (mut active_devices, mut failed_devices) = (Vec::new(), Vec::new());
    for result in maybe_devices {
        match result {
            Ok(device) => active_devices.push(device),
            Err(error) => failed_devices.push(error),
        }
    }
    #[cfg(debug_assertions)]
    active_devices.push(Box::new(FakeDevice::new()));
    DeviceList::from_devices(active_devices, failed_devices)
}

async fn get_device_list() -> DeviceList {
    run_in_thread(get_device_list_blocking).await
}

struct BasicSummary {
    name: String,
    serial: String,
    path: String,
    firmware: String,
    interface: Interface,
}

fn get_summaries_blocking(device_list: Arc<DeviceList>) -> Vec<BasicSummary> {
    device_list
        .active_devices
        .iter()
        .map(|device| BasicSummary {
            name: device.model_number().unwrap_or("Unknown".into()),
            serial: device.serial_number().unwrap_or("Unknown".into()),
            path: device.path().unwrap_or("Unknown".into()),
            firmware: device.firmware_revision().unwrap_or("Unknown".into()),
            interface: device.interface().unwrap_or(Interface::Other),
        })
        .collect()
}

async fn get_summaries(device_list: Arc<DeviceList>) -> Vec<BasicSummary> {
    run_in_thread(move || get_summaries_blocking(device_list)).await
}

pub async fn update_device_list(app_state: Rc<AtomicBorrow<AppState>>) {
    let device_list = get_device_list().await;
    let device_list = app_state.with_mut(move |app_state| {
        app_state.device_list = Arc::new(device_list);
        app_state.window.set_additional_drives(AdditionalDrivesModel::new(
            app_state
                .device_list
                .failed_devices
                .iter()
                .map(|(path, error)| (path.clone(), error.to_string()))
                .collect(),
            app_state
                .device_list
                .failed_enumeration
                .as_ref()
                .map(|error| error.to_string())
                .unwrap_or(String::new()),
            app_state
                .device_list
                .failed_enumeration
                .as_ref()
                .map(|_| ContentStatus::Error)
                .unwrap_or(ContentStatus::Success),
        ));
        app_state.device_list.clone()
    });
    let summaries = get_summaries(device_list.clone()).await;
    app_state.with_mut(move |app_state| {
        let summaries: Vec<_> = summaries
            .into_iter()
            .map(|summary| {
                SummaryModel::new(
                    summary.name,
                    summary.serial,
                    summary.path,
                    summary.firmware,
                    summary.interface.to_string(),
                    ContentStatus::Loading,
                    String::new(),
                    vec![],
                    vec![],
                    vec![],
                    vec![],
                )
            })
            .collect();
        app_state.summaries = ModelRc::new(VecModel::from(summaries));
        app_state.window.set_summaries(app_state.summaries.clone());
    });
}
