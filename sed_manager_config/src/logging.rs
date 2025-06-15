//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::fs::File;

fn get_level_env() -> Option<tracing::Level> {
    let Ok(value) = std::env::var("RUST_LOG") else { return None };
    value.parse().ok()
}

fn get_level_cli() -> Option<tracing::Level> {
    const LOG_ARG_PREFIX: &str = "--log=";
    let log_arg = std::env::args().find(|arg| arg.starts_with(LOG_ARG_PREFIX))?;
    let value = &log_arg[LOG_ARG_PREFIX.len()..];
    value.parse().ok()
}

pub fn get_level() -> Option<tracing::Level> {
    match (get_level_env(), get_level_cli()) {
        (Some(x), Some(y)) => Some(std::cmp::max(x, y)),
        (mx, my) => mx.or(my),
    }
}

pub fn init(level: tracing::Level) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    let temp_dir = std::env::temp_dir();
    let log_file_path = temp_dir.join("sed-manager.log");
    println!("Log level set to `{level}`");
    if let Ok(file) = File::create(log_file_path.as_path()) {
        let (non_blocking, guard) = tracing_appender::non_blocking(file);
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_writer(non_blocking)
            .with_max_level(level)
            .with_target(false);
        let _ = tracing::subscriber::set_global_default(subscriber.finish());
        Some(guard)
    } else {
        eprintln!("Failed to create log file: {}", log_file_path.as_path().to_str().unwrap_or("?"));
        None
    }
}
