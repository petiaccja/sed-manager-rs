use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;

use opentelemetry_otlp::{WithExportConfig, WithTonicConfig};

use tonic::metadata::{MetadataKey, MetadataMap};

use crate::otlp::{
    Error::{self, InvalidProtocol},
    HeaderError,
};

pub enum ExporterProtocol {
    Grpc,
    Http,
}

pub struct NetworkConfig {
    endpoint: String,
    protocol: ExporterProtocol,
    headers: HashMap<String, String>,
}

impl NetworkConfig {
    pub fn get() -> Result<Self, Error> {
        Ok(Self { endpoint: get_endpoint()?, protocol: get_protocol()?, headers: get_headers() })
    }
}

pub fn create_network_exporter(config: Option<NetworkConfig>) -> Result<opentelemetry_otlp::SpanExporter, Error> {
    let NetworkConfig { endpoint, protocol, headers } = match config {
        Some(config) => config,
        None => NetworkConfig::get()?,
    };

    let mut metadata = MetadataMap::new();
    for (key, value) in headers {
        let key = MetadataKey::from_str(&key).map_err(HeaderError::InvalidKey)?;
        metadata.insert(key, value.parse().map_err(HeaderError::InvalidValue)?);
    }

    Ok(match protocol {
        ExporterProtocol::Grpc => opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_metadata(metadata)
            .with_endpoint(endpoint)
            .with_timeout(Duration::from_millis(500))
            .build()?,
        ExporterProtocol::Http => opentelemetry_otlp::SpanExporter::builder()
            .with_http()
            .with_endpoint(endpoint)
            .with_timeout(Duration::from_millis(500))
            .build()?,
    })
}

fn get_endpoint() -> Result<String, Error> {
    std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").map_err(|_| Error::EndpointNotSpecified)
}

fn get_protocol() -> Result<ExporterProtocol, Error> {
    match std::env::var("OTEL_EXPORTER_OTLP_PROTOCOL") {
        Ok(value) => match value.to_lowercase().as_str() {
            "http" => Ok(ExporterProtocol::Http),
            "grpc" => Ok(ExporterProtocol::Grpc),
            _ => Err(Error::InvalidProtocol(value)),
        },
        Err(_) => Err(InvalidProtocol("<none>".into())),
    }
}

fn get_headers() -> HashMap<String, String> {
    std::env::var("OTEL_EXPORTER_OTLP_HEADERS")
        .map(|value| {
            value
                .split(' ')
                .filter(|entry| !entry.is_empty())
                .filter_map(|entry| entry.split_once('='))
                .map(|(key, value)| (key.to_owned(), value.to_owned()))
                .collect()
        })
        .unwrap_or_default()
}
