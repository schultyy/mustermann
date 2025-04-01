use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{
    self,
    logs::LoggerProvider,
    runtime,
    trace::{self, RandomIdGenerator},
    Resource,
};
use opentelemetry_semantic_conventions::resource::SERVICE_NAME;
use tonic::metadata::{MetadataMap, MetadataValue};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;

use opentelemetry::{trace::TracerProvider as _, KeyValue};

pub fn setup_otlp(
    endpoint: &str,
    service_name: &str,
) -> Result<LoggerProvider, Box<dyn std::error::Error>> {
    let mut map = MetadataMap::with_capacity(3);

    map.insert("x-application", service_name.parse().unwrap());
    map.insert_bin(
        "trace-proto-bin",
        MetadataValue::from_bytes(b"[binary data]"),
    );

    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .with_metadata(map)
        .build()?;

    // Then pass it into provider builder
    let provider = opentelemetry_sdk::trace::TracerProvider::builder()
        .with_batch_exporter(otlp_exporter, opentelemetry_sdk::runtime::Tokio)
        .with_config(
            trace::Config::default()
                // .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default())
                .with_max_events_per_span(64)
                .with_max_attributes_per_span(16)
                .with_max_events_per_span(16)
                .with_resource(Resource::new(vec![KeyValue::new(
                    SERVICE_NAME,
                    service_name.to_string(),
                )])),
        )
        .build();
    let tracer = provider.tracer("mustermann_root_tracer");

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
        .with(tracing_subscriber::fmt::layer())
        .with(layer)
        .with(OpenTelemetryLayer::new(tracer))
        .init();
    Ok(logger_provider)
}
