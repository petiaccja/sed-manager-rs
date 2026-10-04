use std::fs::File;
use std::sync::Mutex;
use std::{future::Future, io, path::Path};

use opentelemetry_proto::tonic::collector::trace::v1::ExportTraceServiceRequest;
use opentelemetry_proto::transform::common::tonic::ResourceAttributesWithSchema;
use opentelemetry_proto::transform::trace::tonic::group_spans_by_resource_and_scope;
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::error::OTelSdkError;
use opentelemetry_sdk::{
    error::OTelSdkResult,
    trace::{SpanData, SpanExporter},
};
use std::io::Write;

use crate::otlp::Error;

pub fn create_file_exporter(path: impl AsRef<Path>) -> Result<FileSpanExporter, Error> {
    FileSpanExporter::open(path).map_err(Into::into)
}

#[derive(Debug)]
pub struct FileSpanExporter {
    file: Mutex<File>,
    resource: Resource,
}

impl FileSpanExporter {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, io::Error> {
        let file = File::open(path)?;
        let file = Mutex::new(file);
        Ok(Self { file, resource: Resource::builder_empty().build() })
    }
}

impl SpanExporter for FileSpanExporter {
    fn export(&self, batch: Vec<SpanData>) -> impl Future<Output = OTelSdkResult> + Send {
        async {
            let resource = ResourceAttributesWithSchema::default();
            let resource_spans = group_spans_by_resource_and_scope(batch, &resource);
            let request = ExportTraceServiceRequest { resource_spans };
            let json_str =
                serde_json::to_string(&request).map_err(|err| OTelSdkError::InternalFailure(err.to_string()))?;
            let mut file = match self.file.lock() {
                Ok(file) => file,
                Err(panicked) => panicked.into_inner(),
            };
            file.write_all(json_str.as_ref()).map_err(|err| OTelSdkError::InternalFailure(err.to_string()))
        }
    }

    fn force_flush(&self) -> OTelSdkResult {
        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(panicked) => panicked.into_inner(),
        };
        file.flush().map_err(|err| OTelSdkError::InternalFailure(err.to_string()))
    }

    fn set_resource(&mut self, resource: &opentelemetry_sdk::Resource) {
        self.resource = resource.clone();
    }

    fn shutdown(&self) -> OTelSdkResult {
        self.force_flush()
    }

    fn shutdown_with_timeout(&self, _timeout: std::time::Duration) -> OTelSdkResult {
        self.force_flush()
    }
}
