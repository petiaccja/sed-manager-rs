#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

mod backend;
mod configuration;
mod device_list;
mod frontend;
mod logging;
mod settings;
mod troubleshooting;
mod ui;
mod utility;

use backend::Backend;
use core::error::Error;
use frontend::Frontend;
use settings::remove_markdown_directives;
use slint::ComponentHandle;
use std::rc::Rc;
use utility::PeekCell;

fn main() -> Result<(), Box<dyn Error>> {
    let _guard = logging::init();
    let backend = Rc::new(PeekCell::new(Backend::new()));

    // Load settings.
    let settings = Rc::new(PeekCell::new(settings::load().unwrap_or(settings::Settings::default())));

    // Configure callbacks.
    let _ = slint::BackendSelector::new().backend_name("winit".into()).renderer_name("skia".into()).select();
    let app_window = ui::AppWindow::new()?;
    let frontend = Frontend::new(app_window.clone_strong());

    configuration::set_callbacks(backend.clone(), frontend.clone());
    troubleshooting::set_callbacks(backend.clone(), frontend.clone());
    device_list::set_callbacks(backend.clone(), frontend.clone());
    settings::set_callbacks(settings.clone(), frontend.clone());
    app_window.on_quit(|| {
        let _ = slint::quit_event_loop();
    });

    // Set parameters for the about page and the license.
    let ui_settings = app_window.global::<ui::SettingsState>();
    let license = include_str!("../../LICENSE.md");
    ui_settings.set_license_text(remove_markdown_directives(license).into());

    // Refresh device list right after starting.
    let _ = app_window.show();
    app_window.global::<ui::DeviceListState>().invoke_list();
    if !settings.peek(|settings| settings.license_accepted) {
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
