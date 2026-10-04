use std::io;
use std::path::Path;

use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::ExporterBuildError;
use opentelemetry_sdk::trace::SpanExporter;
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use tonic::metadata::errors::{InvalidMetadataKey, InvalidMetadataValue};
use tracing::Metadata;
use tracing_subscriber::{Layer, Registry, layer::Context};

mod file;
mod network;

pub use file::create_file_exporter;
pub use network::create_network_exporter;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the OTLP endpoint is not specified, please set the `OTEL_EXPORTER_OTLP_ENDPOINT` env var")]
    EndpointNotSpecified,
    #[error("invalid headers: {0}")]
    InvalidHeaders(#[from] HeaderError),
    #[error("invalid protocol: {0}")]
    InvalidProtocol(String),
    #[error("invalid headers: {0}")]
    ExporterBuild(#[from] ExporterBuildError),
    #[error("{0}")]
    Io(#[from] io::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum HeaderError {
    #[error("{0}")]
    InvalidKey(#[from] InvalidMetadataKey),
    #[error("{0}")]
    InvalidValue(#[from] InvalidMetadataValue),
}

pub fn create_layer(exporter: impl SpanExporter + 'static, trace_name: &str) -> impl Layer<Registry> {
    // Build the SDK tracer provider.
    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(
            Resource::builder()
                .with_attributes(
                    [
                        KeyValue::new("service.name", trace_name.to_owned()),
                        KeyValue::new("service.version", "0.1.0"),
                    ]
                    .into_iter(),
                )
                .build(),
        )
        .build();

    // Build the tracing layer.
    let tracer = provider.tracer(trace_name.to_owned());
    tracing_opentelemetry::layer().with_tracer(tracer).with_filter(CrateFilter)
}

struct CrateFilter;

impl<S> tracing_subscriber::layer::Filter<S> for CrateFilter {
    fn enabled(&self, meta: &Metadata<'_>, _cx: &Context<'_, S>) -> bool {
        meta.file().map(|file| Path::new(file).is_relative()).unwrap_or(false)
    }
}
