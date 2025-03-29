use clap::{Parser, ValueEnum};

use logger::log_demo_data;
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
use tracer::simulate_checkout_process;
use tracing::{debug, info, Level};
use tracing_opentelemetry::OpenTelemetryLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;

use opentelemetry::{trace::TracerProvider as _, KeyValue};

mod config;
mod logger;
mod tracer;
/// CLI tool for pattern matching
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// OTLP endpoint URL when using OTLP logging
    #[arg(short, long, default_value = "http://localhost:4317")]
    otlp_endpoint: String,
    /// Enable logging and specify the output destination
    #[arg(long, value_enum)]
    log: Option<LogOutput>,

    /// Enable metrics collection
    #[arg(long, default_value_t = false)]
    metrics: bool,

    /// Enable tracing
    #[arg(long, default_value_t = false)]
    tracing: bool,
}

/// Logging output destinations
#[derive(Debug, Clone, ValueEnum, PartialEq, Eq)]
enum LogOutput {
    /// Log to standard output
    Stdout,
    /// Log to OpenTelemetry Protocol (OTLP) endpoint
    Otlp,
}

fn setup_otlp_logger(endpoint: &str) -> Result<LoggerProvider, Box<dyn std::error::Error>> {
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

fn setup_tracer(endpoint: &str) -> Result<(), Box<dyn std::error::Error>> {
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args = Args::parse();
    let mut logger_provider: Option<LoggerProvider> = None;

    // Configure logging based on the selected output
    match args.log {
        Some(LogOutput::Stdout) => {
            tracing_subscriber::registry()
                .with(tracing_subscriber::filter::EnvFilter::from_default_env())
                .with(tracing_subscriber::fmt::layer())
                .init();

            info!("Logging initialized with stdout output");
        }
        Some(LogOutput::Otlp) => {
            // This is a stub for now
            logger_provider = Some(setup_otlp_logger(&args.otlp_endpoint)?);
            info!("OTLP initialized");
        }
        None => {}
    }

    // Log feature activation status
    if let Some(log_output) = &args.log {
        info!("Logging enabled with {:?} output", log_output);
    }

    if args.metrics {
        info!("Metrics will come soon :rocket:");
        // Stub for metrics implementation
    }

    if args.tracing {
        info!("Tracing enabled");
        // Stub for tracing implementation
        setup_tracer(&args.otlp_endpoint)?;
        tracer::run_checkout_simulation();
        simulate_checkout_process();
    }

    // Generate some demo log data if stdout logging is enabled
    if args.log == Some(LogOutput::Stdout) {
        debug!("This is a debug message (only visible with --verbose)");
        info!("This is an info message");

        log_demo_data()
    } else if args.log == Some(LogOutput::Otlp) {
        log_demo_data()
    } else {
        eprintln!("No CLI arguments provided");
    }

    opentelemetry::global::shutdown_tracer_provider();
    if let Some(logger_provider) = logger_provider {
        logger_provider.shutdown()?;
    }
    Ok(())
}
