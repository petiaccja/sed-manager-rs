#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod callbacks;
mod device_list;
mod ui;
mod utility;

use core::error::Error;
use slint::ComponentHandle;
use std::rc::Rc;

use app_state::AppState;
use utility::AtomicBorrow;

fn set_callbacks(app_window: &ui::AppWindow, app_state: Rc<AtomicBorrow<AppState>>) {
    let copy = app_state.clone();
    app_window.on_update_devices(move || {
        let _ = slint::spawn_local(callbacks::update_devices(copy.clone()));
    });
    let copy = app_state.clone();
    app_window.on_update_device_discovery(move |device_idx| {
        let _ = slint::spawn_local(callbacks::update_device_discovery(copy.clone(), device_idx as usize));
    });
    let copy = app_state.clone();
    app_window.on_reset_session(move |device_idx| {
        let _ = slint::spawn_local(callbacks::reset_session(copy.clone(), device_idx as usize));
    });
    let copy = app_state.clone();
    app_window.on_take_ownership(move |device_idx, new_password| {
        let _ = slint::spawn_local(callbacks::take_ownership(copy.clone(), device_idx as usize, new_password.into()));
    });
    let copy = app_state.clone();
    app_window.on_activate_locking(move |device_idx, sid_password, new_admin1_password| {
        let _ = slint::spawn_local(callbacks::activate_locking(
            copy.clone(),
            device_idx as usize,
            sid_password.into(),
            Some(new_admin1_password.into()),
        ));
    });
    let copy = app_state.clone();
    app_window.on_query_ranges(move |device_idx, password| {
        let _ = slint::spawn_local(callbacks::query_ranges(copy.clone(), device_idx as usize, password.into()));
    });
    let copy = app_state.clone();
    app_window.on_revert(move |device_idx, use_psid, password, include_admin| {
        let _ = slint::spawn_local(callbacks::revert(
            copy.clone(),
            device_idx as usize,
            use_psid,
            password.into(),
            include_admin,
        ));
    });
}

fn main() -> Result<(), Box<dyn Error>> {
    let _ = slint::BackendSelector::new().backend_name("winit".into()).renderer_name("skia".into()).select();

    let app_window = ui::AppWindow::new()?;
    let app_state = Rc::new(AtomicBorrow::new(AppState::new(app_window.clone_strong())));
    set_callbacks(&app_window, app_state.clone());
    let _ = slint::spawn_local(callbacks::update_devices(app_state.clone())); // Update the device list on startup.
    app_window.run()?;

    Ok(())
}
