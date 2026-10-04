use std::io;
use std::sync::Mutex;

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

pub fn create_file_exporter<F>(file: F) -> Result<FileSpanExporter<F>, Error>
where
    F: Write + Send + core::fmt::Debug,
{
    FileSpanExporter::new(file).map_err(Into::into)
}

/// Export OTLP spans into a JSONL file. You can pass any stream that implements
/// [`Write`], not just files.
#[derive(Debug)]
pub struct FileSpanExporter<F> {
    file: Mutex<F>,
    resource: Resource,
}

impl<F> FileSpanExporter<F> {
    pub fn new(file: F) -> Result<Self, io::Error> {
        let file = Mutex::new(file);
        Ok(Self { file, resource: Resource::builder_empty().build() })
    }
}

impl<F> SpanExporter for FileSpanExporter<F>
where
    F: Write + Send + core::fmt::Debug,
{
    async fn export(&self, batch: Vec<SpanData>) -> OTelSdkResult {
        let resource = ResourceAttributesWithSchema::from(&self.resource);
        let resource_spans = group_spans_by_resource_and_scope(batch, &resource);
        let request = ExportTraceServiceRequest { resource_spans };
        let mut json_str =
            serde_json::to_string(&request).map_err(|err| OTelSdkError::InternalFailure(err.to_string()))?;
        json_str.push('\n');
        let mut file = match self.file.lock() {
            Ok(file) => file,
            Err(panicked) => panicked.into_inner(),
        };
        file.write_all(json_str.as_ref()).map_err(|err| OTelSdkError::InternalFailure(err.to_string()))
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

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, SystemTime};

    use googletest::{assert_that, matchers::*};
    use opentelemetry::trace::{Span as _, SpanId, TraceContextExt as _, TraceId, Tracer as _, TracerProvider as _};
    use opentelemetry::{Context, KeyValue};
    use opentelemetry_sdk::trace::{IdGenerator, SdkTracerProvider};
    use serde_json::{Value, json};

    use super::*;

    /// A writer that keeps the written bytes accessible after the exporter takes ownership of it.
    #[derive(Debug, Clone, Default)]
    struct SharedBuffer(Arc<Mutex<Vec<u8>>>);

    impl Write for SharedBuffer {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            self.0.lock().unwrap().write(buf)
        }

        fn flush(&mut self) -> io::Result<()> {
            Ok(())
        }
    }

    /// Generates sequential IDs so that the output is deterministic.
    #[derive(Debug, Default)]
    struct SequentialIdGenerator {
        next_trace_id: AtomicU64,
        next_span_id: AtomicU64,
    }

    impl IdGenerator for SequentialIdGenerator {
        fn new_trace_id(&self) -> TraceId {
            TraceId::from(u128::from(self.next_trace_id.fetch_add(1, Ordering::Relaxed) + 1))
        }

        fn new_span_id(&self) -> SpanId {
            SpanId::from(self.next_span_id.fetch_add(1, Ordering::Relaxed) + 1)
        }
    }

    fn timestamp(secs: u64) -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(secs)
    }

    #[test]
    fn export_format() {
        let buffer = SharedBuffer::default();
        let exporter = FileSpanExporter::new(buffer.clone()).unwrap();
        let provider = SdkTracerProvider::builder()
            .with_simple_exporter(exporter)
            .with_id_generator(SequentialIdGenerator::default())
            .with_resource(Resource::builder_empty().with_service_name("file_test").build())
            .build();
        let tracer = provider.tracer("file_test");

        // The simple processor exports each span in its own batch when the span ends.
        let parent = tracer.span_builder("parent").with_start_time(timestamp(1)).start(&tracer);
        let parent_cx = Context::current_with_span(parent);
        let mut child = tracer
            .span_builder("child")
            .with_start_time(timestamp(2))
            .with_attributes([KeyValue::new("key", 42)])
            .start_with_context(&tracer, &parent_cx);
        child.end_with_timestamp(timestamp(3));
        parent_cx.span().end_with_timestamp(timestamp(4));
        provider.shutdown().unwrap();

        let output = String::from_utf8(buffer.0.lock().unwrap().clone()).unwrap();
        let lines: Vec<Value> = output.lines().map(|line| serde_json::from_str(line).unwrap()).collect();

        let expected = json!([
            // The child ends first, so it's exported first.
            {
                "resourceSpans": [{
                    "resource": {
                        "attributes": [{ "key": "service.name", "value": { "stringValue": "file_test" } }],
                        "droppedAttributesCount": 0,
                        "entityRefs": [],
                    },
                    "schemaUrl": "",
                    "scopeSpans": [{
                        "schemaUrl": "",
                        "scope": { "attributes": [], "droppedAttributesCount": 0, "name": "file_test", "version": "" },
                        "spans": [{
                            "attributes": [{ "key": "key", "value": { "intValue": "42" } }],
                            "droppedAttributesCount": 0,
                            "droppedEventsCount": 0,
                            "droppedLinksCount": 0,
                            "endTimeUnixNano": "3000000000",
                            "events": [],
                            "flags": 257,
                            "kind": 1,
                            "links": [],
                            "name": "child",
                            "parentSpanId": "0000000000000001",
                            "spanId": "0000000000000002",
                            "startTimeUnixNano": "2000000000",
                            "status": { "code": 0, "message": "" },
                            "traceId": "00000000000000000000000000000001",
                            "traceState": "",
                        }],
                    }],
                }],
            },
            {
                "resourceSpans": [{
                    "resource": {
                        "attributes": [{ "key": "service.name", "value": { "stringValue": "file_test" } }],
                        "droppedAttributesCount": 0,
                        "entityRefs": [],
                    },
                    "schemaUrl": "",
                    "scopeSpans": [{
                        "schemaUrl": "",
                        "scope": { "attributes": [], "droppedAttributesCount": 0, "name": "file_test", "version": "" },
                        "spans": [{
                            "attributes": [],
                            "droppedAttributesCount": 0,
                            "droppedEventsCount": 0,
                            "droppedLinksCount": 0,
                            "endTimeUnixNano": "4000000000",
                            "events": [],
                            "flags": 257,
                            "kind": 1,
                            "links": [],
                            "name": "parent",
                            "parentSpanId": "",
                            "spanId": "0000000000000001",
                            "startTimeUnixNano": "1000000000",
                            "status": { "code": 0, "message": "" },
                            "traceId": "00000000000000000000000000000001",
                            "traceState": "",
                        }],
                    }],
                }],
            },
        ]);
        assert_that!(output, ends_with("\n"));
        assert_that!(Value::from(lines), eq(&expected));
    }
}
