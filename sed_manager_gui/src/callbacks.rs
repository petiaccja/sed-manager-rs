use core::future::Future;
use std::rc::Rc;
use std::sync::Arc;

use sed_manager::applications::{get_locking_admins, get_locking_sp, get_lookup};
use sed_manager::device::Device;
use sed_manager::messaging::discovery::FeatureCode;
use sed_manager::rpc::{ErrorEvent as RPCErrorEvent, ErrorEventExt};
use sed_manager::spec::column_types::{AuthorityRef, LockingRangeRef, SPRef};
use sed_manager::spec::table_id;
use sed_manager::tper::{discover, Session};
use sed_manager::{applications, spec};

use crate::app_state::{AppState, SyncDeviceIdentity};
use crate::device_list::DeviceList;
use crate::ui::{self, ActionResult};
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

/// Clean up currently open sessions for the selected device, reset result structures.
pub async fn reset_session(app_state: Rc<AtomicBorrow<AppState>>, device_idx: usize) {
    app_state.with_mut(|app_state| {
        app_state.set_action_result(device_idx, ui::ActionResult::error("".into()));
        app_state.clear_locking_range_errors(device_idx);
        app_state.clear_locking_ranges(device_idx);
    });

    if let Some(session) = app_state.with_mut(|app_state| app_state.take_session(device_idx)) {
        if let Some(session) = Arc::into_inner(session) {
            let _ = session.end_session().await;
        }
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

pub async fn start_persistent_session(
    app_state: Rc<AtomicBorrow<AppState>>,
    device_idx: usize,
    sp: SPRef,
    authority: Option<AuthorityRef>,
    password: Option<String>,
) -> Result<Arc<Session>, applications::error::Error> {
    if let Some(tper) = app_state.with_mut(|app_state| app_state.get_tper(device_idx)) {
        let session = Arc::new(tper.start_session(sp, authority, password.as_ref().map(|s| s.as_bytes())).await?);
        let old_session = app_state.with_mut(|app_state| app_state.set_session(device_idx, session.clone()));
        if let Some(old_session) = old_session {
            if let Some(old_session) = Arc::into_inner(old_session) {
                let _ = old_session.end_session().await;
            }
        }
        Ok(session)
    } else {
        Err(RPCErrorEvent::NotSupported.as_error().into())
    }
}

pub async fn start_query_ranges_session(
    app_state: Rc<AtomicBorrow<AppState>>,
    device_idx: usize,
    password: String,
) -> Result<Arc<Session>, applications::error::Error> {
    let ssc = app_state.with(|app_state| {
        let discovery = app_state.get_discovery(device_idx).ok_or(applications::error::Error::NoAvailableSSC)?;
        let ssc = discovery.get_primary_ssc().ok_or(applications::error::Error::IncompatibleSSC)?;
        Ok::<FeatureCode, applications::error::Error>(ssc.feature_code())
    })?;
    let locking_sp = get_locking_sp(ssc)?;
    let authority = get_locking_admins(ssc)?.nth(1).ok_or(applications::error::Error::IncompatibleSSC)?;
    start_persistent_session(app_state, device_idx, locking_sp, Some(authority), Some(password)).await
}

pub async fn query_ranges(app_state: Rc<AtomicBorrow<AppState>>, device_idx: usize, password: String) {
    app_state.with_mut(|app_state| app_state.set_action_result(device_idx, ActionResult::loading()));
    let session = match start_query_ranges_session(app_state.clone(), device_idx, password).await {
        Ok(session) => session,
        Err(err) => {
            app_state
                .with_mut(|app_state| app_state.set_action_result(device_idx, ActionResult::error(err.to_string())));
            return;
        }
    };
    app_state.with_mut(|app_state| app_state.set_action_result(device_idx, ActionResult::success()));

    let locking_ranges = match session.next(table_id::LOCKING, None, None).await {
        Ok(locking_ranges) => locking_ranges,
        Err(err) => {
            app_state.with_mut(|app_state| app_state.append_locking_range_error(device_idx, err.to_string()));
            return;
        }
    };

    let ssc = app_state
        .with(|app_state| {
            let discovery = app_state.get_discovery(device_idx).ok_or(applications::error::Error::NoAvailableSSC)?;
            let ssc = discovery.get_primary_ssc().ok_or(applications::error::Error::IncompatibleSSC)?;
            Ok::<FeatureCode, applications::error::Error>(ssc.feature_code())
        })
        .unwrap_or(FeatureCode::Unrecognized);

    let sp = get_locking_sp(ssc).ok();
    let lookup = get_lookup(ssc);
    for range in locking_ranges.iter() {
        let name = lookup.by_uid(*range, sp.map(|sp| sp.as_uid())).unwrap_or(format!("{:16X}", range.as_u64()));
        let ui_range = ui::LockingRange::new(name, 0, 0, false, false, false, false);
        app_state.with_mut(|app_state| app_state.append_locking_range(device_idx, ui_range));
    }
}
