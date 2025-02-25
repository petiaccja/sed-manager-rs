use core::future::Future;
use std::rc::Rc;

use sed_manager::device::Device;
use sed_manager::rpc::{ErrorEvent as RPCErrorEvent, ErrorEventExt};
use sed_manager::tper::discover;
use sed_manager::{applications, spec};

use crate::app_state::{AppState, SyncDeviceIdentity};
use crate::device_list::DeviceList;
use crate::ui;
use crate::utility::{run_in_thread, AtomicBorrow, Versioned};

async fn update_device_list(app_state: Rc<AtomicBorrow<AppState>>) {
    let snapshot = app_state.with(|app_state| app_state.get_device_list().snapshot());
    let fresh = DeviceList::query().await;
    app_state.with_mut(|app_state| {
        snapshot.run_if_current(app_state.get_device_list().current(), || match fresh {
            Ok(value) => app_state.set_device_list(value),
            Err(error) => app_state.set_device_list_error(error),
        })
    });
}

async fn update_device_descriptions(app_state: Rc<AtomicBorrow<AppState>>) {
    let (devices, device_snap) =
        app_state.with(|app_state| (app_state.get_device_list().arc(), app_state.get_device_list().snapshot()));

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
        device_snap
            .run_if_current(app_state.get_device_list().current(), || app_state.set_device_identities(identities));
    });
}

pub async fn update_device_discovery(app_state: Rc<AtomicBorrow<AppState>>, device_idx: usize) {
    fn get_device(app_state: &AppState, device_idx: usize) -> Option<&Versioned<dyn Device>> {
        app_state.get_device_list().as_ref().ok().map(|devs| devs.devices.get(device_idx)).flatten()
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

async fn update_device_tabs(app_state: Rc<AtomicBorrow<AppState>>) {
    let (devices, device_snap) =
        app_state.with(|app_state| (app_state.get_device_list().arc(), app_state.get_device_list().snapshot()));

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
        device_snap.run_if_current(app_state.get_device_list().current(), || {
            app_state.set_tabs(names);
        });
    });
}

pub async fn update_devices(app_state: Rc<AtomicBorrow<AppState>>) {
    update_device_list(app_state.clone()).await;
    update_device_descriptions(app_state.clone()).await;
    update_device_tabs(app_state.clone()).await;
    let num_devs =
        app_state.with(|app_state| app_state.get_device_list().as_ref().map(|devs| devs.devices.len()).unwrap_or(0));
    for dev_idx in 0..num_devs {
        update_device_discovery(app_state.clone(), dev_idx).await;
    }
}

pub async fn one_click_action(
    app_state: Rc<AtomicBorrow<AppState>>,
    device_idx: usize,
    future_result: impl Future<Output = Result<(), applications::error::Error>>,
) {
    app_state.with_mut(|app_state| {
        app_state.set_action_result(device_idx, ui::ActionResult::loading());
    });

    let action_result = match future_result.await {
        Ok(_) => ui::ActionResult::success(),
        Err(err) => ui::ActionResult::error(err.to_string()),
    };

    app_state.with_mut(|app_state| {
        app_state.set_action_result(device_idx, action_result);
    });
}

async fn take_ownership_impl(
    app_state: Rc<AtomicBorrow<AppState>>,
    device_idx: usize,
    new_password: String,
) -> Result<(), applications::error::Error> {
    if let Some(tper) = app_state.with_mut(|app_state| app_state.get_tper(device_idx)) {
        applications::take_ownership(&*tper, new_password.as_bytes()).await
    } else {
        Err(RPCErrorEvent::NotSupported.as_error().into())
    }
}

async fn activate_locking_impl(
    app_state: Rc<AtomicBorrow<AppState>>,
    device_idx: usize,
    sid_password: String,
    new_admin1_password: Option<String>,
) -> Result<(), applications::error::Error> {
    if let Some(tper) = app_state.with_mut(|app_state| app_state.get_tper(device_idx)) {
        applications::activate_locking(
            &*tper,
            sid_password.as_bytes(),
            new_admin1_password.as_ref().map(|s| s.as_bytes()),
        )
        .await
    } else {
        Err(RPCErrorEvent::NotSupported.as_error().into())
    }
}

async fn revert_impl(
    app_state: Rc<AtomicBorrow<AppState>>,
    device_idx: usize,
    use_psid: bool,
    password: String,
    include_admin: bool,
) -> Result<(), applications::error::Error> {
    if let Some(tper) = app_state.with_mut(|app_state| app_state.get_tper(device_idx)) {
        let security_providers = app_state
            .with(|app_state| {
                app_state.get_discovery(device_idx).map(|discovery| {
                    let ssc = discovery.get_primary_ssc()?;
                    let admin_sp = applications::get_admin_sp(ssc.feature_code()).ok()?;
                    let locking_sp = applications::get_locking_sp(ssc.feature_code()).ok()?;
                    Some((admin_sp, locking_sp))
                })
            })
            .flatten();

        if let Some((admin_sp, locking_sp)) = security_providers {
            let authority = match use_psid {
                true => spec::psid::admin::authority::PSID,
                false => spec::core::authority::SID,
            };
            let sp = match include_admin || use_psid {
                true => admin_sp,
                false => locking_sp,
            };
            let result = applications::revert(&*tper, authority, password.as_bytes(), sp).await;
            result
        } else {
            Err(RPCErrorEvent::NotSupported.as_error().into())
        }
    } else {
        Err(RPCErrorEvent::NotSupported.as_error().into())
    }
}

pub async fn take_ownership(app_state: Rc<AtomicBorrow<AppState>>, device_idx: usize, new_password: String) {
    let future = take_ownership_impl(app_state.clone(), device_idx, new_password);
    one_click_action(app_state, device_idx, future).await;
}

pub async fn activate_locking(
    app_state: Rc<AtomicBorrow<AppState>>,
    device_idx: usize,
    sid_password: String,
    new_admin1_password: Option<String>,
) {
    let future = activate_locking_impl(app_state.clone(), device_idx, sid_password, new_admin1_password);
    one_click_action(app_state, device_idx, future).await;
}

pub async fn revert(
    app_state: Rc<AtomicBorrow<AppState>>,
    device_idx: usize,
    use_psid: bool,
    password: String,
    include_admin: bool,
) {
    let future = revert_impl(app_state.clone(), device_idx, use_psid, password, include_admin);
    one_click_action(app_state, device_idx, future).await;
}
