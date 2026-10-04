#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::sync::Arc;

use sed_async::{PolyRuntime, TokioRuntime};
use sed_manager::Host;
use sed_manager_gui::{app::App, toast::ToastQueue};
use sed_manager_gui_slint::{self as ui};
use sed_telemetry::otlp;
use slint::ComponentHandle as _;
use tracing::{error, warn_span};
use tracing_subscriber::{Registry, layer::SubscriberExt as _, util::SubscriberInitExt as _};

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let runtime = Arc::new(PolyRuntime::Tokio(TokioRuntime::multi_threaded(Some(1))?));
    let _otlp_flush_guard = init_tracing();
    let host = Arc::new(Host::new(runtime.clone()));
    let ui = ui::MainWindow::new()?;
    let notification_queue = ToastQueue::new(ui.clone_strong());
    let main_app = App::new(ui.clone_strong(), notification_queue.clone(), host, runtime);
    main_app.scan(true);
    ui.show()?;
    slint::run_event_loop_until_quit()?;
    ui.hide()?;
    Ok(())
}

fn init_tracing() -> Option<otlp::FlushGuard> {
    let registry = Registry::default();

    // Attempt to set up the network exporter.
    match otlp::create_network_exporter(None) {
        Ok(exporter) => {
            let (layer, sdk_tracer_provider) = otlp::LayerBuilder::new()
                .with_batch_exporter(exporter)
                .with_service_name(env!("CARGO_PKG_NAME"))
                .with_service_version(env!("CARGO_PKG_VERSION"))
                .build();
            let registry = registry.with(layer);
            registry.init();
            Some(otlp::FlushGuard::new(sdk_tracer_provider))
        }
        Err(err) => {
            // Fall back to an stdout exporter.
            let registry = registry.with(tracing_subscriber::fmt::layer());
            registry.init();
            // Log that setting up the network exporter failed.
            warn_span!("create_network_exporter").in_scope(|| error!("{err}"));
            None
        }
    }
}
