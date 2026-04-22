use std::collections::HashMap;
use std::path::Path;
use std::str::FromStr;
use std::sync::Mutex;
use std::time::Duration;

use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use tonic::metadata::{MetadataKey, MetadataMap};
use tracing::Metadata;
use tracing_subscriber::layer::{Context, SubscriberExt as _};
use tracing_subscriber::util::SubscriberInitExt;
use tracing_subscriber::{Layer, Registry};

static GLOBAL_SUBSCRIBER_SET: Mutex<bool> = Mutex::new(false);

#[cfg(feature = "test-utils")]
mod test_utils {
    #[rstest::fixture]
    pub fn with_tracing() -> WithTracing {
        let mut global_subscriber_set = super::GLOBAL_SUBSCRIBER_SET.lock().unwrap();
        if *global_subscriber_set {
            WithTracing { _guard: super::TracingGuard { provider: None } }
        } else {
            *global_subscriber_set = true;
            match super::create_otlp_provider() {
                Ok(provider) => WithTracing { _guard: super::init_otlp_subscriber(provider) },
                Err(_) => WithTracing { _guard: super::init_stdout_subscriber() },
            }
        }
    }

    pub struct WithTracing {
        _guard: super::TracingGuard,
    }

    impl std::fmt::Debug for WithTracing {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "with_telemetry")
        }
    }
}

#[cfg(feature = "test-utils")]
pub use test_utils::*;

pub fn get_otel_exporter_endpoint() -> Result<String, std::env::VarError> {
    std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
}

pub fn get_otel_exporter_headers() -> Option<HashMap<String, String>> {
    std::env::var("OTEL_EXPORTER_OTLP_HEADERS").ok().map(|value| {
        value
            .split(' ')
            .filter(|entry| !entry.is_empty())
            .filter_map(|entry| entry.split_once('='))
            .map(|(key, value)| (key.to_owned(), value.to_owned()))
            .collect()
    })
}

pub fn create_otlp_provider() -> Result<SdkTracerProvider, Box<dyn std::error::Error + Send + Sync>> {
    let endpoint = get_otel_exporter_endpoint()?;

    let mut metadata = MetadataMap::new();
    if let Some(headers) = get_otel_exporter_headers() {
        for (key, value) in headers {
            if let Ok(key) = MetadataKey::from_str(&key) {
                metadata.insert(key, value.parse()?);
            } else {
                eprintln!("invalid OTLP header key: {key}");
            }
        }
    }

    // Create OTLP span exporter
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_metadata(metadata)
        .with_tls_config(tonic::transport::ClientTlsConfig::new().with_native_roots())
        .with_endpoint(endpoint)
        .with_timeout(Duration::from_millis(500))
        .build()?;

    // Build the tracer provider
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_attributes(vec![
                    KeyValue::new("service.name", "sed-manager-test"),
                    KeyValue::new("service.version", "0.1.0"),
                ])
                .build(),
        )
        .build();

    Ok(provider)
}

pub fn init_otlp_subscriber(provider: SdkTracerProvider) -> TracingGuard {
    opentelemetry::global::set_tracer_provider(provider.clone());

    let tracer = provider.tracer("sed-manager-test");
    let telemetry = tracing_opentelemetry::layer().with_tracer(tracer).with_filter(CrateFilter);
    let subscriber = Registry::default().with(telemetry);
    subscriber.init();

    TracingGuard { provider: Some(provider) }
}

pub fn init_stdout_subscriber() -> TracingGuard {
    let subscriber = tracing_subscriber::fmt();
    subscriber.init();

    TracingGuard { provider: None }
}

pub struct TracingGuard {
    provider: Option<SdkTracerProvider>,
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Some(provider) = &self.provider {
            let _ = provider.force_flush();
            let _ = provider.shutdown_with_timeout(Duration::from_secs(5));
        }
    }
}

struct CrateFilter;

impl<S> tracing_subscriber::layer::Filter<S> for CrateFilter {
    fn enabled(&self, meta: &Metadata<'_>, _cx: &Context<'_, S>) -> bool {
        meta.file().map(|file| Path::new(file).is_relative()).unwrap_or(false)
    }
}
