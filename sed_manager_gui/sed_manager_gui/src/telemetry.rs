//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::{
    fs, io,
    path::{Path, PathBuf},
};

use sed_telemetry::otlp;
use tracing::{error, level_filters::LevelFilter, warn_span};
use tracing_subscriber::filter::{Builder as EnvFilterBuilder, ParseError};
use tracing_subscriber::{EnvFilter, Registry, layer::SubscriberExt as _, util::SubscriberInitExt as _};

pub fn init_telemetry(log_file: Option<PathBuf>, log_level: Option<&str>) -> Option<otlp::FlushGuard> {
    let registry = Registry::default();
    let mut error_messages = Vec::new();
    let (env_filter, maybe_filter_error) = create_env_filter(log_level);
    error_messages.extend(maybe_filter_error.map(|msg| ("create_env_filter", msg)));

    let maybe_network_exporter = otlp::create_network_exporter(None);
    let maybe_file_exporter = {
        let path = log_file.unwrap_or_else(default_log_file);
        let maybe_log_file = create_log_file(path).map_err(otlp::Error::Io);
        maybe_log_file.and_then(otlp::create_file_exporter)
    };

    let flush_guard = if maybe_network_exporter.is_ok() || maybe_file_exporter.is_ok() {
        let layer_builder = otlp::LayerBuilder::new()
            .with_service_name(env!("CARGO_PKG_NAME"))
            .with_service_version(env!("CARGO_PKG_VERSION"));

        let layer_builder = match maybe_network_exporter {
            Ok(exporter) => layer_builder.with_batch_exporter(exporter),
            Err(err) => {
                error_messages.push(("create_network_exporter", err.to_string()));
                layer_builder
            }
        };

        let layer_builder = match maybe_file_exporter {
            Ok(exporter) => layer_builder.with_batch_exporter(exporter),
            Err(err) => {
                error_messages.push(("create_file_exporter", err.to_string()));
                layer_builder
            }
        };

        let (layer, sdk_tracer_provider) = layer_builder.build();

        let registry = registry.with(layer).with(env_filter);
        registry.init();
        Some(otlp::FlushGuard::new(sdk_tracer_provider))
    } else {
        // Fall back to an stdout exporter.
        let registry = registry.with(tracing_subscriber::fmt::layer()).with(env_filter);
        registry.init();
        None
    };

    // Log failures that occurred while setting up logging itself.
    for (task, message) in &error_messages {
        warn_span!("set_up_logging", task).in_scope(|| error!("{message}"));
    }

    flush_guard
}

fn env_filter_builder() -> EnvFilterBuilder {
    EnvFilter::builder().with_default_directive(LevelFilter::INFO.into())
}

/// Validates the log filter directives given on the command line.
pub fn parse_log_filter(directives: &str) -> Result<String, ParseError> {
    env_filter_builder().parse(directives).map(|_| directives.to_owned())
}

/// Creates the log filter from the command line directives, or from `RUST_LOG` if there are none.
///
/// Invalid directives in `RUST_LOG` are not fatal, as the variable may have been set for other
/// applications. In that case, the default filter is used and the error is returned.
fn create_env_filter(directives: Option<&str>) -> (EnvFilter, Option<String>) {
    let builder = env_filter_builder();
    if let Some(directives) = directives {
        (builder.parse_lossy(directives), None)
    } else {
        match builder.from_env() {
            Ok(filter) => (filter, None),
            Err(err) => {
                let filter = builder.parse_lossy("");
                let message = format!("invalid `RUST_LOG`, using `{filter}` instead: {err}");
                (filter, Some(message))
            }
        }
    }
}

fn create_log_file(path: impl AsRef<Path>) -> Result<fs::File, io::Error> {
    if let Some(dir) = path.as_ref().parent() {
        std::fs::create_dir_all(dir)?;
    }
    fs::OpenOptions::new().create_new(true).write(true).open(path)
}

fn default_log_file() -> PathBuf {
    let application = env!("CARGO_PKG_NAME");
    let timestamp = chrono::Local::now().format("%Y-%m-%d_%H-%M-%S");
    let pid = std::process::id();
    std::env::temp_dir().join("sed_manager").join(format!("{application}_{timestamp}_{pid}.jsonl"))
}
