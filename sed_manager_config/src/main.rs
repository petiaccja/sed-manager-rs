#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod backend;
mod configuration;
mod device_list;
mod frontend;
mod troubleshooting;
mod ui;
mod utility;

use backend::Backend;
use core::error::Error;
use frontend::Frontend;
use slint::ComponentHandle;
use std::{fs::File, rc::Rc};
use utility::PeekCell;

fn init_logging() -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let temp_dir = std::env::temp_dir();
    let log_file_path = temp_dir.join("sed-manager.log");
    if let Ok(file) = File::create(log_file_path.as_path()) {
        let (non_blocking, guard) = tracing_appender::non_blocking(file);
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(non_blocking)
            .with_max_level(tracing::Level::DEBUG)
            .with_target(false);
        let _ = tracing::subscriber::set_global_default(subscriber.finish());
        Some(guard)
    } else {
        eprintln!("Could not write to log file: {}", log_file_path.as_path().to_str().unwrap_or("?"));
        None
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let _guard = init_logging();
    let backend = Rc::new(PeekCell::new(Backend::new()));

    let _ = slint::BackendSelector::new().backend_name("winit".into()).renderer_name("skia".into()).select();
    let app_window = ui::AppWindow::new()?;
    let frontend = Frontend::new(app_window.clone_strong());

    configuration::set_callbacks(backend.clone(), frontend.clone());
    troubleshooting::set_callbacks(backend.clone(), frontend.clone());
    device_list::set_callbacks(backend.clone(), frontend.clone());

    // Refresh device list right after starting.
    app_window.global::<ui::DeviceListState>().invoke_list();

    // Display GUI.
    app_window.run()?;

    Ok(())
}
