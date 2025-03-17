use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{logs::LoggerProvider, runtime, trace::RandomIdGenerator, Resource};
use tonic::metadata::{MetadataMap, MetadataValue};
use tracing::Level;
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::filter::filter_fn;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

use opentelemetry::{trace::TracerProvider as _, KeyValue};

// Original OTLP logger setup for compatibility
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

// Original tracer setup for compatibility
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
        .with_id_generator(RandomIdGenerator::default())
        .with_max_events_per_span(64)
        .with_max_attributes_per_span(16)
        .with_max_events_per_span(16)
        .with_resource(Resource::new(vec![KeyValue::new(
            "service.name",
            "mustermann",
        )]))
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

/// Setup combined logging for both internal telemetry and muster data
pub fn setup_combined_logging(
    otlp_endpoint: Option<&str>,
) -> Result<LoggerProvider, Box<dyn std::error::Error>> {
    // Set up a common environment filter
    let env_filter = EnvFilter::from_default_env().add_directive("mustermann=info".parse()?);

    // Build a registry with a default fmt layer for console output
    let registry = tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt::layer().with_ansi(true));

    // If an OTLP endpoint is provided, add an OTLP layer for muster logs
    if let Some(endpoint) = otlp_endpoint {
        // Create the OTLP exporter
        let exporter = LogExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .with_metadata({
                let mut map = MetadataMap::new();
                map.insert("x-application", "mustermann".parse()?);
                map
            })
            .build()?;

        // Build a logger provider
        let provider = LoggerProvider::builder()
            .with_batch_exporter(exporter, runtime::Tokio)
            .with_resource(Resource::new(vec![KeyValue::new(
                "service.name",
                "mustermann",
            )]))
            .build();

        // Create a filter for muster data (only logs with log_source field)
        let muster_filter = filter_fn(|metadata| metadata.fields().field("log_source").is_some());

        // Create the OTLP bridge and add it to the registry
        let otlp_layer = OpenTelemetryTracingBridge::new(&provider).with_filter(muster_filter);

        // Initialize the registry with the OTLP layer
        registry.with(otlp_layer).init();

        Ok(provider)
    } else {
        // No OTLP endpoint, just initialize the registry as is
        registry.init();

        // Return a dummy provider since we don't have a real one
        Ok(LoggerProvider::builder().build())
    }
}

/// This function is maintained for backward compatibility
pub fn setup_internal_telemetry() -> Result<(), Box<dyn std::error::Error>> {
    eprintln!(
        "Warning: setup_internal_telemetry is deprecated, use setup_combined_logging instead"
    );

    // Simple subscriber for backward compatibility
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(true)
        .with_ansi(true)
        .init();

    Ok(())
}

/// This function is maintained for backward compatibility
pub fn setup_muster_logging(
    _otlp_endpoint: Option<&str>,
) -> Result<LoggerProvider, Box<dyn std::error::Error>> {
    eprintln!("Warning: setup_muster_logging is deprecated, use setup_combined_logging instead");

    // Return a dummy provider for backward compatibility
    Ok(LoggerProvider::builder().build())
}
