use std::rc::Rc;
use std::sync::Arc;

use sed_manager::messaging::discovery::Discovery;
use slint::{ComponentHandle as _, Model};

use sed_manager::device::{list_physical_drives, open_device, Device, Error as DeviceError};
use sed_manager::fake_device::FakeDevice;
use sed_manager::rpc::Error as RPCError;

use crate::backend::Backend;
use crate::frontend::Frontend;
use crate::native_data::NativeDeviceIdentity;
use crate::ui;
use crate::utility::{into_vec_model, run_in_thread, PeekCell};

pub fn clear(frontend: &Frontend) {
    frontend.with(|window| {
        let dev_list_state = window.global::<ui::DeviceListState>();
        dev_list_state.set_descriptions(into_vec_model(vec![]));
        dev_list_state.set_tab_names(into_vec_model(vec![]));
        dev_list_state.set_unavailable_devices(into_vec_model(vec![]));
    });
    crate::configuration::clear(frontend);
    crate::troubleshooting::clear(frontend);
}

pub fn set_callbacks(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    set_callback_for_list(backend.clone(), frontend.clone());
    set_callback_for_discover(backend.clone(), frontend.clone());
    set_callback_for_release_session(backend.clone(), frontend.clone());
}

fn set_callback_for_list(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let dev_list_state = window.global::<ui::DeviceListState>();

        dev_list_state.on_list(move || {
            let frontend = frontend.clone();
            let backend = backend.clone();
            clear(&frontend);
            set_status(&frontend, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = list(backend).await;
                if let Ok(DispDeviceList { identities, unavailable }) = result {
                    let identities: Vec<_> = identities.into_iter().map(|x| ui::DeviceIdentity::from(x)).collect();
                    let unavailable: Vec<_> =
                        unavailable.into_iter().map(|x| ui::UnavailableDevice::new(x.0, x.1.to_string())).collect();
                    crate::configuration::init(&frontend, identities.len());
                    crate::troubleshooting::init(&frontend, identities.len());
                    set_identities(&frontend, identities);
                    set_unavailable(&frontend, unavailable);
                    set_tabs(&frontend);
                    set_status(&frontend, ui::ExtendedStatus::success());
                } else {
                    set_status(&frontend, ui::ExtendedStatus::from_result(result));
                }
            });
        });
    });
}

fn set_callback_for_discover(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let dev_list_state = window.global::<ui::DeviceListState>();

        dev_list_state.on_discover(move |device_idx| {
            let frontend = frontend.clone();
            let backend = backend.clone();
            let device_idx = device_idx as usize;
            set_discovery_status(&frontend, device_idx, ui::ExtendedStatus::loading());
            let _ = slint::spawn_local(async move {
                let result = discover(backend, device_idx).await;
                if let Ok(discovery) = result {
                    let discovery_ui = ui::DeviceDiscovery::from_discovery(&discovery);
                    let activity_support = ui::ActivitySupport::from_discovery(&discovery);
                    let geometry = ui::DeviceGeometry::from_discovery(&discovery);
                    set_discovery(&frontend, device_idx, discovery_ui, activity_support, geometry);
                    set_discovery_status(&frontend, device_idx, ui::ExtendedStatus::success());
                } else {
                    set_discovery_status(&frontend, device_idx, ui::ExtendedStatus::from_result(result));
                }
            });
        });
    });
}

fn set_callback_for_release_session(backend: Rc<PeekCell<Backend>>, frontend: Frontend) {
    frontend.clone().with(|window| {
        let dev_list_state = window.global::<ui::DeviceListState>();

        dev_list_state.on_release_session(move |device_idx| {
            let device_idx = device_idx as usize;
            let backend = backend.clone();
            let _ = slint::spawn_local(async move {
                release_session(backend, device_idx).await;
            });
        });
    });
}

pub struct HwDeviceList {
    pub opened: Vec<Arc<dyn Device>>,
    pub unavailable: Vec<(String, DeviceError)>,
}

pub struct DispDeviceList {
    pub identities: Vec<NativeDeviceIdentity>,
    pub unavailable: Vec<(String, DeviceError)>,
}

async fn list(backend: Rc<PeekCell<Backend>>) -> Result<DispDeviceList, DeviceError> {
    let devices = run_in_thread(list_blocking).await?;
    let mut identities = Vec::<NativeDeviceIdentity>::new();
    for device in &devices.opened {
        identities.push(get_identity(device.clone()).await.into());
    }
    backend.peek_mut(|backend| backend.set_devices(devices.opened));
    Ok(DispDeviceList { identities, unavailable: devices.unavailable })
}

async fn discover(backend: Rc<PeekCell<Backend>>, device_idx: usize) -> Result<Discovery, RPCError> {
    let device = backend.peek_mut(|backend| backend.get_device(device_idx)).ok_or(DeviceError::DeviceNotFound)?;
    let discovery = run_in_thread(move || sed_manager::rpc::discover(&*device)).await?;
    backend.peek_mut(|backend| backend.set_discovery(device_idx, discovery.clone()));
    Ok(discovery)
}

async fn release_session(backend: Rc<PeekCell<Backend>>, device_idx: usize) {
    if let Some(session) = backend.peek_mut(|backend| backend.take_session(device_idx)) {
        if let Some(session) = Arc::into_inner(session) {
            let _ = session.end_session().await;
        }
    }
}

fn set_identities(frontend: &Frontend, identities: Vec<ui::DeviceIdentity>) {
    frontend.with(|window| {
        let dev_list_state = window.global::<ui::DeviceListState>();
        let descriptions = identities
            .into_iter()
            .map(|identity| {
                ui::DeviceDescription::new(
                    identity,
                    ui::ExtendedStatus::loading(),
                    ui::DeviceDiscovery::empty(),
                    ui::ActivitySupport::none(),
                    ui::DeviceGeometry::unknown(),
                )
            })
            .collect();
        dev_list_state.set_descriptions(into_vec_model(descriptions));
    });
}

fn set_unavailable(frontend: &Frontend, unavailable: Vec<ui::UnavailableDevice>) {
    frontend.with(|window| {
        let dev_list_state = window.global::<ui::DeviceListState>();
        dev_list_state.set_unavailable_devices(into_vec_model(unavailable));
    });
}

fn set_status(frontend: &Frontend, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let dev_list_state = window.global::<ui::DeviceListState>();
        dev_list_state.set_extended_status(status);
    });
}

fn set_tabs(frontend: &Frontend) {
    frontend.with(|window| {
        let dev_list_state = window.global::<ui::DeviceListState>();
        let mut tab_names = Vec::new();
        for desc in dev_list_state.get_descriptions().iter() {
            tab_names.push(desc.identity.name);
        }
        if dev_list_state.get_unavailable_devices().row_count() != 0 {
            tab_names.push("Unavailable devices".into());
        }
        dev_list_state.set_tab_names(into_vec_model(tab_names));
    });
}

fn set_discovery_status(frontend: &Frontend, device_idx: usize, status: ui::ExtendedStatus) {
    frontend.with(|window| {
        let dev_list_state = window.global::<ui::DeviceListState>();
        let descs = dev_list_state.get_descriptions();
        if let Some(mut desc) = descs.row_data(device_idx) {
            desc.discovery_status = status;
            descs.set_row_data(device_idx, desc);
        }
    });
}

fn set_discovery(
    frontend: &Frontend,
    device_idx: usize,
    discovery: ui::DeviceDiscovery,
    activity_support: ui::ActivitySupport,
    geometry: ui::DeviceGeometry,
) {
    frontend.with(|window| {
        let dev_list_state = window.global::<ui::DeviceListState>();
        let descs = dev_list_state.get_descriptions();
        if let Some(mut desc) = descs.row_data(device_idx) {
            desc.discovery = discovery;
            desc.activity_support = activity_support;
            desc.geometry = geometry;
            descs.set_row_data(device_idx, desc);
        }
    });
}

fn list_blocking() -> Result<HwDeviceList, DeviceError> {
    let device_paths = list_physical_drives()?;
    let maybe_devices: Vec<_> = device_paths
        .into_iter()
        .map(move |path| open_device(&path).map_err(|error| (path, error)))
        .collect();
    let (mut devices, mut unavailable_devices) = (Vec::new(), Vec::new());
    for result in maybe_devices {
        match result {
            Ok(device) => devices.push(Arc::from(device)),
            Err(error) => unavailable_devices.push(error),
        }
    }
    #[cfg(debug_assertions)]
    devices.push(Arc::new(FakeDevice::new()) as Arc<dyn Device>);
    Ok(HwDeviceList { opened: devices, unavailable: unavailable_devices })
}

pub async fn get_identity(device: Arc<dyn Device>) -> NativeDeviceIdentity {
    run_in_thread(move || NativeDeviceIdentity {
        name: device.model_number().unwrap_or("Unknown model".into()),
        serial: device.serial_number().unwrap_or("Unknown serial".into()),
        path: device.path().unwrap_or("Unknown path".into()),
        firmware: device.firmware_revision().unwrap_or("Unknown firmware".into()),
        interface: device.interface().map(|x| x.to_string()).unwrap_or("Unknown interface".into()),
    })
    .await
}
