use std::sync::OnceLock;

use tracing::{error, warn_span};
use tracing_subscriber::{Registry, layer::SubscriberExt, util::SubscriberInitExt};

use crate::otlp;

static INITIALIZED: OnceLock<()> = OnceLock::new();

pub fn with_tracing() {
    INITIALIZED.get_or_init(|| {
        let registry = Registry::default();

        // Attempt to set up the network exporter.
        match otlp::create_network_exporter(None) {
            Ok(exporter) => {
                let layer = otlp::create_layer(exporter, "with_tracing");
                let registry = registry.with(layer);
                registry.init();
            }
            Err(err) => {
                // Fall back to an stdout exporter.
                let registry = registry.with(tracing_subscriber::fmt::layer());
                registry.init();
                // Log that setting up the network exporter failed.
                warn_span!("create_network_exporter").in_scope(|| error!("{err}"));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use sed_telemetry_macros::with_tracing;

    #[test]
    #[with_tracing]
    fn blocking() {}
}
