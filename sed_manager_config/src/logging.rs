use std::fs::File;

pub fn init() -> Option<tracing_appender::non_blocking::WorkerGuard> {
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
