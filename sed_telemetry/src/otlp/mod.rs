use std::collections::HashMap;
use std::io;
use std::path::Path;

use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::ExporterBuildError;
use opentelemetry_sdk::trace::{SpanExporter, TracerProviderBuilder};
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

pub struct LayerBuilder {
    trace_provider_builder: TracerProviderBuilder,
    attributes: HashMap<String, String>,
}

impl LayerBuilder {
    pub fn new() -> Self {
        let attributes = [
            ("service.name".to_owned(), env!("CARGO_PKG_NAME").to_owned()),
            ("service.version".to_owned(), env!("CARGO_PKG_VERSION").to_owned()),
        ]
        .into();
        Self { trace_provider_builder: SdkTracerProvider::builder(), attributes }
    }

    pub fn with_batch_exporter(self, exporter: impl SpanExporter + 'static) -> Self {
        Self { trace_provider_builder: self.trace_provider_builder.with_batch_exporter(exporter), ..self }
    }

    pub fn with_service(mut self, name: String, version: String) -> Self {
        self.attributes.insert("service.name".into(), name);
        self.attributes.insert("service.version".into(), version);
        self
    }

    pub fn build(self) -> (impl Layer<Registry>, SdkTracerProvider) {
        let Self { trace_provider_builder, attributes } = self;
        let service_name = attributes.get("service.name").cloned().unwrap_or(env!("CARGO_PKG_NAME").to_owned());
        let resource = Resource::builder()
            .with_attributes(attributes.into_iter().map(|(key, value)| KeyValue::new(key, value)))
            .build();
        let trace_provider_builder = trace_provider_builder.with_resource(resource);

        let sdk_tracer_provider = trace_provider_builder.build();
        let tracer = sdk_tracer_provider.tracer(service_name);
        let layer = tracing_opentelemetry::layer().with_tracer(tracer).with_filter(CrateFilter);
        (layer, sdk_tracer_provider)
    }
}

impl Default for LayerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct FlushGuard {
    sdk_tracer_provider: SdkTracerProvider,
}
impl FlushGuard {
    pub fn new(sdk_tracer_provider: SdkTracerProvider) -> Self {
        Self { sdk_tracer_provider }
    }
}

impl Drop for FlushGuard {
    fn drop(&mut self) {
        let _ = self.sdk_tracer_provider.force_flush();
    }
}

struct CrateFilter;

impl<S> tracing_subscriber::layer::Filter<S> for CrateFilter {
    fn enabled(&self, meta: &Metadata<'_>, _cx: &Context<'_, S>) -> bool {
        meta.file().map(|file| Path::new(file).is_relative()).unwrap_or(false)
    }
}
