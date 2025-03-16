use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{
    self,
    logs::LoggerProvider,
    runtime,
    trace::{self, RandomIdGenerator},
    Resource,
};
use tonic::metadata::{MetadataMap, MetadataValue};
use tracing::Level;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;

use opentelemetry::{trace::TracerProvider as _, KeyValue};

pub fn setup_otlp_logger(endpoint: &str) -> Result<LoggerProvider, Box<dyn std::error::Error>> {
    let exporter = LogExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .with_metadata(MetadataMap::new())
        .build()?;
    let logger_provider = LoggerProvider::builder()
        .with_batch_exporter(exporter, runtime::Tokio)
        .build();
    let layer = OpenTelemetryTracingBridge::new(&logger_provider);

    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::EnvFilter::from_default_env())
        .with(layer)
        .init();
    Ok(logger_provider)
}

pub fn setup_tracer(endpoint: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut map = MetadataMap::with_capacity(3);

    map.insert("x-application", "mustermann".parse().unwrap());
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
                    "service.name",
                    "mustermann",
                )])),
        )
        .build();
    let tracer = provider.tracer("mustermann_root_tracer");
    tracing_subscriber::registry()
        .with(tracing_subscriber::filter::LevelFilter::from_level(
            Level::INFO,
        ))
        .with(tracing_subscriber::fmt::layer())
        .with(OpenTelemetryLayer::new(tracer))
        .init();
    Ok(())
}
