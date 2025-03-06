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

    let _ = slint::BackendSelector::new().backend_name("winit".into()).renderer_name("skia".into()).select();
    let backend = Rc::new(PeekCell::new(Backend::new()));
    let app_window = ui::AppWindow::new()?;
    let async_state = AsyncState::new(backend, app_window.as_weak());

    // Set up callbacks.
    async_state.on_list_devices(Backend::list_devices);
    async_state.on_discover(Backend::discover);
    async_state.on_cleanup_session(Backend::cleanup_session);
    async_state.on_take_ownership(Backend::take_ownership);
    async_state.on_activate_locking(Backend::activate_locking);
    async_state.on_login_locking_admin(Backend::login_locking_admin);
    async_state.on_list_locking_ranges(Backend::list_locking_ranges);
    async_state.on_set_locking_range(Backend::set_locking_range);
    async_state.on_erase_locking_range(Backend::erase_locking_range);
    async_state.on_list_locking_users(Backend::list_locking_users);
    async_state.on_revert(Backend::revert);
    async_state.on_reset_stack(Backend::reset_stack);

    // Refresh device list right after starting.
    app_window.global::<ui::State>().invoke_list_devices();

    // Display GUI.
    app_window.run()?;

    Ok(())
}
