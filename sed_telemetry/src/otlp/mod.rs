//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::collections::HashMap;
use std::io;
use std::path::Path;

use opentelemetry::KeyValue;
use opentelemetry::trace::TracerProvider as _;
use opentelemetry_otlp::ExporterBuildError;
use opentelemetry_sdk::error::OTelSdkError;
use opentelemetry_sdk::trace::{SpanExporter, TracerProviderBuilder};
use opentelemetry_sdk::{Resource, trace::SdkTracerProvider};
use opentelemetry_semantic_conventions::attribute::{
    HOST_ARCH, OS_BUILD_ID, OS_NAME, OS_TYPE, OS_VERSION, SERVICE_NAME, SERVICE_VERSION,
};
use opentelemetry_semantic_conventions::trace::PROCESS_EXECUTABLE_NAME;
use tracing::Metadata;
use tracing_subscriber::{Layer, Registry, layer::Context};

mod file;
mod network;

pub use file::create_file_exporter;
pub use network::create_network_exporter;

const SERVICE_BUILD_TYPE: &str = "service.build_type";

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("the OTLP endpoint is not specified, please set the `OTEL_EXPORTER_OTLP_ENDPOINT` env var")]
    EndpointNotSpecified,
    #[error("the OTLP protocol `{0}` is not supported; use `http/protobuf` or `http/json`")]
    UnsupportedProtocol(String),
    #[error("the OTLP endpoint is unreachable: {0}")]
    EndpointUnreachable(OTelSdkError),
    #[error("failed to build exporter: {0}")]
    ExporterBuild(#[from] ExporterBuildError),
    #[error("{0}")]
    Io(#[from] io::Error),
}

pub struct LayerBuilder {
    trace_provider_builder: TracerProviderBuilder,
    attributes: HashMap<String, String>,
}

impl LayerBuilder {
    pub fn new() -> Self {
        Self { trace_provider_builder: SdkTracerProvider::builder(), attributes: collect_default_attributes() }
    }

    pub fn with_batch_exporter(self, exporter: impl SpanExporter + 'static) -> Self {
        Self { trace_provider_builder: self.trace_provider_builder.with_batch_exporter(exporter), ..self }
    }

    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.attributes.insert(SERVICE_NAME.into(), name.into());
        self
    }

    pub fn with_service_version(mut self, version: impl Into<String>) -> Self {
        self.attributes.insert(SERVICE_VERSION.into(), version.into());
        self
    }

    pub fn build(self) -> (impl Layer<Registry>, SdkTracerProvider) {
        let Self { trace_provider_builder, attributes } = self;
        let service_name = attributes.get(SERVICE_NAME).cloned().unwrap_or(env!("CARGO_PKG_NAME").to_owned());
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

fn collect_default_attributes() -> HashMap<String, String> {
    [
        Some((SERVICE_NAME.to_owned(), env!("CARGO_PKG_NAME").to_owned())),
        Some((SERVICE_VERSION.to_owned(), env!("CARGO_PKG_VERSION").to_owned())),
        Some((HOST_ARCH.to_owned(), std::env::consts::ARCH.to_owned())),
        Some((OS_TYPE.to_owned(), std::env::consts::OS.to_owned())),
        std::env::current_exe().ok().and_then(|path| {
            path.file_name()
                .map(|file_name| (PROCESS_EXECUTABLE_NAME.to_owned(), file_name.to_string_lossy().into()))
        }),
        Some((SERVICE_BUILD_TYPE.to_owned(), if cfg!(debug_assertions) { "debug" } else { "release" }.to_owned())),
        os_name().map(|value| (OS_NAME.into(), value)),
        os_version().map(|value| (OS_VERSION.into(), value)),
        os_build_id().map(|value| (OS_BUILD_ID.into(), value)),
    ]
    .into_iter()
    .flatten()
    .collect()
}

fn os_name() -> Option<String> {
    #[cfg(target_os = "windows")]
    return None;
    #[cfg(target_os = "linux")]
    return Some(rustix::system::uname().sysname().to_string_lossy().into_owned());
    #[allow(unreachable_code)]
    None
}

fn os_version() -> Option<String> {
    #[cfg(target_os = "windows")]
    return None;
    #[cfg(target_os = "linux")]
    return Some(rustix::system::uname().release().to_string_lossy().into_owned());
    #[allow(unreachable_code)]
    None
}

fn os_build_id() -> Option<String> {
    #[cfg(target_os = "windows")]
    return None;
    #[cfg(target_os = "linux")]
    return Some(rustix::system::uname().version().to_string_lossy().into_owned());
    #[allow(unreachable_code)]
    None
}

#[derive(Debug, Clone)]
#[must_use]
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
        if let Err(err) = self.sdk_tracer_provider.force_flush() {
            eprintln!("failed to flush OTLP tracer provider: {err}");
        }
    }
}

struct CrateFilter;

impl<S> tracing_subscriber::layer::Filter<S> for CrateFilter {
    fn enabled(&self, meta: &Metadata<'_>, _cx: &Context<'_, S>) -> bool {
        meta.file().map(|file| Path::new(file).is_relative()).unwrap_or(false)
    }
}
