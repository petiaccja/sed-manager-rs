use std::sync::OnceLock;

use tracing::{error, warn_span};
use tracing_subscriber::{Registry, layer::SubscriberExt, util::SubscriberInitExt};

use crate::otlp;

static INITIALIZED: OnceLock<Option<otlp::FlushGuard>> = OnceLock::new();

#[must_use]
pub fn with_tracing() -> Option<otlp::FlushGuard> {
    let maybe_guard = INITIALIZED.get_or_init(|| {
        let registry = Registry::default();

        // Attempt to set up the network exporter.
        match otlp::create_network_exporter(None) {
            Ok(exporter) => {
                let (layer, sdk_tracer_provider) = otlp::LayerBuilder::new()
                    .with_batch_exporter(exporter)
                    .with_service("with_tracing", env!("CARGO_PKG_VERSION"))
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
    });

    maybe_guard.clone()
}

#[cfg(test)]
mod tests {
    use sed_telemetry_macros::with_tracing;

    #[test]
    #[with_tracing]
    fn blocking() {}
}
