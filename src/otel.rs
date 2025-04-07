use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::Resource;
use opentelemetry_sdk::{logs::LoggerProvider, runtime};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tonic::metadata::MetadataMap;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;

pub fn setup_otlp(
    endpoint: &str,
    service_name: &str,
) -> Result<LoggerProvider, Box<dyn std::error::Error>> {
    let mut metadata = MetadataMap::new();
    metadata.insert(SERVICE_NAME, service_name.parse().unwrap());
    let exporter = LogExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .with_metadata(metadata)
        .build()?;
    let logger_provider = LoggerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .with_resource(Resource::new_with_defaults(vec![KeyValue::new(
            SERVICE_NAME,
            service_name.to_string(),
        )]))
        .build();
    let layer = OpenTelemetryTracingBridge::new(&logger_provider);

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "INFO".into()),
        )
        .with(tracing_subscriber::fmt::layer().json())
        .with(layer)
        .init();
    Ok(logger_provider)
}
