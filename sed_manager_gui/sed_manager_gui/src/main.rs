#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::Arc;

use sed_async::{PolyRuntime, TokioRuntime};
use sed_manager_gui::{app::App, toast::ToastQueue};
use sed_manager_gui_slint::{self as ui};
use sed_telemetry::{create_otlp_provider, init_otlp_subscriber, init_stdout_subscriber};
use slint::ComponentHandle as _;

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let runtime = Arc::new(PolyRuntime::Tokio(TokioRuntime::multi_threaded(Some(1))?));
    let _tracing_guard = match create_otlp_provider() {
        Ok(provider) => init_otlp_subscriber(provider),
        Err(_) => init_stdout_subscriber(),
    };
    let ui = ui::MainWindow::new()?;
    let notification_queue = ToastQueue::new(ui.clone_strong());
    let main_app = App::new(ui.clone_strong(), notification_queue.clone(), runtime);
    main_app.scan(true);
    ui.run()?;
    Ok(())
}
