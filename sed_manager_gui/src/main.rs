#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod device_list;
//mod discovery;
mod data;
mod ui;
mod utility;

use slint::ComponentHandle;
use std::error::Error;
use std::rc::Rc;

use app_state::AppState;
use utility::AtomicBorrow;

fn set_callbacks(app_window: &ui::AppWindow, app_state: Rc<AtomicBorrow<AppState>>) {
    let copy = app_state.clone();
    app_window.on_update_devices(move || {
        let _ = slint::spawn_local(app_state::update_devices(copy.clone()));
    });
    let copy = app_state.clone();
    app_window.on_update_device_discovery(move |device_idx| {
        let _ = slint::spawn_local(app_state::update_device_discovery(copy.clone(), device_idx as usize));
    });
}

fn main() -> Result<(), Box<dyn Error>> {
    let _ = slint::BackendSelector::new().backend_name("winit".into()).renderer_name("skia".into()).select();

    let app_window = ui::AppWindow::new()?;
    let app_state = Rc::new(AtomicBorrow::new(AppState::new(app_window.clone_strong())));
    set_callbacks(&app_window, app_state.clone());
    let _ = slint::spawn_local(app_state::update_devices(app_state.clone())); // Update the device list on startup.
    app_window.run()?;

    Ok(())
}
