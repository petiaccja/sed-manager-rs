#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod async_state;
mod backend;
mod device_list;
mod native_data;
mod ui;
mod utility;

use async_state::AsyncState;
use backend::Backend;
use core::error::Error;
use slint::ComponentHandle;
use std::rc::Rc;
use utility::PeekCell;

fn main() -> Result<(), Box<dyn Error>> {
    let _ = slint::BackendSelector::new().backend_name("winit".into()).renderer_name("skia".into()).select();
    let backend = Rc::new(PeekCell::new(Backend::new()));
    let app_window = ui::AppWindow::new()?;
    let async_state = AsyncState::new(backend, app_window.as_weak());

    // Set up callbacks.
    async_state.on_list_devices(Backend::list_devices);
    async_state.on_discover(Backend::discover);

    // Refresh device list right after starting.
    app_window.global::<ui::State>().invoke_list_devices();

    // Display GUI.
    app_window.run()?;

    Ok(())
}
