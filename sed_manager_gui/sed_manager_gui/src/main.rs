#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::{path::PathBuf, sync::Arc};

use clap::Parser;
use sed_async::{PolyRuntime, TokioRuntime};
use sed_manager::Host;
use sed_manager_gui::{
    app::App,
    telemetry::{init_telemetry, parse_log_filter},
    toast::ToastQueue,
};
use sed_manager_gui_slint::{self as ui};
use slint::ComponentHandle as _;

#[derive(Debug, clap::Parser)]
struct Args {
    #[arg(
        long,
        help = "The path to where the OpenTelemetry traces are written in JSONL. If not given, a file is chosen in the temporary folder."
    )]
    log_file: Option<PathBuf>,
    #[arg(
        long,
        value_parser = parse_log_filter,
        help = "Filter expression for log verbosity. Either a level (e.g. `debug`) or a comma-separated list of `target=level` directives (e.g. `sed_tper=trace,info`). Overrides `RUST_LOG`."
    )]
    log_level: Option<String>,
}

fn main() -> Result<(), Box<dyn core::error::Error>> {
    let args = Args::parse();
    // Declared first so that it's dropped last, after the spans of the runtime's tasks have closed.
    let _otlp_flush_guard = init_telemetry(args.log_file, args.log_level.as_deref());
    let runtime = Arc::new(PolyRuntime::Tokio(TokioRuntime::multi_threaded(Some(1))?));
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
