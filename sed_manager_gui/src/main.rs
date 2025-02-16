#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod atomic_borrow;
mod device_list;
mod discovery;
mod generated;
mod models;
mod utility;

use std::error::Error;
use std::rc::Rc;

use device_list::update_device_list;
use discovery::update_device_discovery;
use generated::AppWindow;
use slint::ComponentHandle;

use app_state::AppState;
use atomic_borrow::AtomicBorrow;

fn set_callbacks(app_window: &AppWindow, app_state: Rc<AtomicBorrow<AppState>>) {
    let copy = app_state.clone();
    app_window.on_update_device_list(move || {
        let _ = slint::spawn_local(update_device_list(copy.clone()));
    });
    let copy = app_state.clone();
    app_window.on_update_device_discovery(move |device_idx| {
        let _ = slint::spawn_local(update_device_discovery(copy.clone(), device_idx as usize));
    });
}

fn main() -> Result<(), Box<dyn Error>> {
    let _ = slint::BackendSelector::new().backend_name("winit".into()).renderer_name("skia".into()).select();

    let app_window = AppWindow::new()?;
    let app_state = Rc::new(AtomicBorrow::new(AppState::new(app_window.clone_strong())));
    set_callbacks(&app_window, app_state.clone());
    let _ = slint::spawn_local(update_device_list(app_state.clone())); // Update the device list on startup.
    app_window.run()?;

    Ok(())
}
