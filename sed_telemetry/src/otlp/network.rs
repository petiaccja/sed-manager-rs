//L-----------------------------------------------------------------------------
//L Copyright (C) Péter Kardos
//L Please refer to the full license distributed with this software.
//L-----------------------------------------------------------------------------

use std::time::Duration;

use opentelemetry_otlp::{Protocol, WithExportConfig};
use opentelemetry_sdk::trace::SpanExporter as _;

use crate::otlp::Error;

const ENDPOINT_VARS: [&str; 2] = [
    "OTEL_EXPORTER_OTLP_TRACES_ENDPOINT",
    "OTEL_EXPORTER_OTLP_ENDPOINT",
];
const PROTOCOL_VARS: [&str; 2] = [
    "OTEL_EXPORTER_OTLP_TRACES_PROTOCOL",
    "OTEL_EXPORTER_OTLP_PROTOCOL",
];

/// Creates an OTLP/HTTP exporter and checks that its endpoint accepts exports.
///
/// To configure the endpoint, use the standardized `OTEL_EXPORTER_OTLP_*` env
/// vars.
///
/// # Errors
///
/// Invalid or unsupported (i.e. gRPC) endpoints or protocols are checked. The
/// endpoint is also probed to see if it accepts exports.
pub fn create_network_exporter() -> Result<opentelemetry_otlp::SpanExporter, Error> {
    check_endpoint()?;
    let exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_http()
        .with_protocol(get_protocol()?)
        .with_timeout(Duration::from_millis(500))
        .build()?;
    probe(&exporter)?;
    Ok(exporter)
}

/// Sends an empty export request to verify the endpoint, headers, and protocol.
///
/// The request is sent from a separate thread because the blocking `reqwest`
/// HTTP client must not be used from within an async runtime, or else it can
/// hang or panic.
fn probe(exporter: &opentelemetry_otlp::SpanExporter) -> Result<(), Error> {
    std::thread::scope(|scope| {
        scope
            .spawn(|| futures::executor::block_on(exporter.export(Vec::new())))
            .join()
            .unwrap_or_else(|panic| std::panic::resume_unwind(panic))
    })
    .map_err(Error::EndpointUnreachable)
}

/// Return the first env var that is defined among the `names`.
fn get_first_env_var(names: &[&str]) -> Option<String> {
    names.iter().filter_map(|name| std::env::var(name).ok()).find(|value| !value.trim().is_empty())
}

fn check_endpoint() -> Result<(), Error> {
    get_first_env_var(&ENDPOINT_VARS).map(|_| ()).ok_or(Error::EndpointNotSpecified)
}

/// Resolves the protocol explicitly, because the exporter would silently fall back to HTTP
/// for `grpc`, and it would default to `http/json` instead of the standard `http/protobuf`.
fn get_protocol() -> Result<Protocol, Error> {
    match get_first_env_var(&PROTOCOL_VARS) {
        Some(protocol) if protocol.trim().eq_ignore_ascii_case("grpc") => Err(Error::UnsupportedProtocol(protocol)),
        Some(protocol) if protocol.trim().eq_ignore_ascii_case("http/json") => Ok(Protocol::HttpJson),
        _ => Ok(Protocol::HttpBinary),
    }
}
