//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::error::Error;
use std::fs::File;
use std::path::{Path, PathBuf};
use tracing_appender::non_blocking::WorkerGuard;

pub struct Log {
    #[allow(unused)]
    worker_guard: Option<WorkerGuard>,
}

impl Log {
    pub fn start(
        log_level: Option<tracing::Level>,
        log_file: Option<impl AsRef<Path>>,
    ) -> Result<Self, Box<dyn Error>> {
        if let Some(log_level) = max_log_level(log_level, read_log_level_env_var()) {
            let tmp_log_file = tmp_log_file();
            let log_file = log_file.as_ref().map(|x| x.as_ref()).unwrap_or(tmp_log_file.as_ref());
            let (non_blocking, worker_guard) = match log_file {
                log_file if log_file == "stdout" => tracing_appender::non_blocking(std::io::stdout()),
                log_file => tracing_appender::non_blocking(File::create(log_file)?),
            };
            let subscriber = tracing_subscriber::fmt()
                .with_ansi(false)
                .with_writer(non_blocking)
                .with_max_level(log_level)
                .with_target(false);
            tracing::subscriber::set_global_default(subscriber.finish())?;
            Ok(Self { worker_guard: Some(worker_guard) })
        } else {
            Ok(Self { worker_guard: None })
        }
    }
}

fn read_log_level_env_var() -> Option<tracing::Level> {
    let Ok(value) = std::env::var("RUST_LOG") else { return None };
    value.parse().ok()
}

pub fn max_log_level(
    log_level_1: Option<tracing::Level>,
    log_level_2: Option<tracing::Level>,
) -> Option<tracing::Level> {
    match (log_level_1, log_level_2) {
        (Some(x), Some(y)) => Some(std::cmp::max(x, y)),
        (mx, my) => mx.or(my),
    }
}

pub fn tmp_log_file() -> PathBuf {
    let temp_dir = std::env::temp_dir();
    temp_dir.join("sed_manager_config.log")
}
