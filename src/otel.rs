use opentelemetry::KeyValue;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{logs::LoggerProvider, runtime, Resource};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tonic::metadata::MetadataMap;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub fn setup_otlp_logger(
    endpoint: &str,
    service_name: &str,
) -> Result<LoggerProvider, Box<dyn std::error::Error>> {
    let exporter = LogExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .with_metadata(MetadataMap::new())
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
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(layer)
        .init();
    Ok(logger_provider)
}
