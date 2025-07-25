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
use license::{get_license_fingerprint, get_plain_license};
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
    let settings = Rc::new(PeekCell::new(settings::load().unwrap_or(settings::Settings::default())));

    // Configure callbacks.
    let _ = slint::BackendSelector::new().backend_name("winit".into()).renderer_name("skia".into()).select();
    let app_window = ui::AppWindow::new()?;
    let frontend = Frontend::new(app_window.clone_strong());

    algorithm::set_callbacks(frontend.clone());
    configuration::set_callbacks(backend.clone(), frontend.clone());
    troubleshooting::set_callbacks(backend.clone(), frontend.clone());
    device_list::set_callbacks(backend.clone(), frontend.clone());
    settings::set_callbacks(settings.clone(), frontend.clone());
    app_window.on_quit(|| {
        let _ = slint::quit_event_loop();
    });

    // Set parameters for the about page and the license.
    let ui_settings = app_window.global::<ui::SettingsState>();
    ui_settings.set_license_text(get_plain_license().into());
    ui_settings.set_license_changed(settings.peek(|settings| settings.accepted_license_fingerprint.is_some()));

    // Refresh device list right after starting.
    let _ = app_window.show();
    app_window.global::<ui::DeviceListState>().invoke_list();

    // Show license agreement if not already accepted.
    let accepted_license_fingerprint = settings.peek(|settings| settings.accepted_license_fingerprint.clone());
    if accepted_license_fingerprint != Some(get_license_fingerprint()) {
        app_window.invoke_show_license();
    }

    // Display GUI.
    app_window.run()?;

    // Save settings if changed.
    if let Err(error) = settings::save(&settings.peek(|settings| settings.clone())) {
        eprint!("Cannot save settings: {error}");
    }

    Ok(())
}
