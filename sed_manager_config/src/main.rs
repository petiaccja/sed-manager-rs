//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod algorithm;
mod backend;
mod configuration;
mod device_list;
mod frontend;
mod license;
mod logging;
mod settings;
mod troubleshooting;
mod ui;
mod utility;

use backend::Backend;
use core::error::Error;
use frontend::Frontend;
use slint::ComponentHandle;
use std::rc::Rc;
use utility::PeekCell;

fn main() -> Result<(), Box<dyn Error>> {
    let log_level = if cfg!(debug_assertions) {
        logging::get_level().or(Some(tracing::Level::DEBUG))
    } else {
        logging::get_level()
    };
    let _guard = log_level.map(|log_level| logging::init(log_level));
    let backend = Rc::new(PeekCell::new(Backend::new()));

    // Load settings.
    let settings = settings::load().unwrap_or(settings::Settings::default());

    // Configure callbacks.
    let _ = slint::BackendSelector::new().backend_name("winit".into()).renderer_name("skia".into()).select();
    let app_window = ui::AppWindow::new()?;
    let frontend = Frontend::new(app_window.clone_strong());

    algorithm::set_callbacks(frontend.clone());
    configuration::set_callbacks(backend.clone(), frontend.clone());
    troubleshooting::set_callbacks(backend.clone(), frontend.clone());
    device_list::set_callbacks(backend.clone(), frontend.clone());
    app_window.on_quit(|| {
        let _ = slint::quit_event_loop();
    });

    // Set parameters for the about page and the license.
    let ui_settings = app_window.global::<ui::SettingsState>();
    settings::set_ui(settings, &ui_settings);

    // Refresh device list right after starting.
    let _ = app_window.show();
    app_window.global::<ui::DeviceListState>().invoke_list();

    // Display GUI.
    app_window.run()?;

    // Save settings if changed.
    let settings = settings::get_ui(&ui_settings);
    if let Err(error) = settings::save(&settings) {
        eprint!("Cannot save settings: {error}");
    }

    Ok(())
}
