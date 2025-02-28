use clap::{Parser, ValueEnum};

use logger::log_demo_data;
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{self, logs::LoggerProvider, runtime};
use tonic::metadata::MetadataMap;
use tracing::{debug, info, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;

mod logger;
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

fn setup_otlp_logger(endpoint: &str) -> Result<(), Box<dyn std::error::Error>> {
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
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args = Args::parse();

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
            setup_otlp_logger(&args.otlp_endpoint)?;
            info!("OTLP initialized");
        }
        None => {
            // Initialize minimal logging for internal use
            tracing_subscriber::fmt().with_max_level(Level::WARN).init();
        }
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

    Ok(())
}
