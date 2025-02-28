use clap::{Parser, ValueEnum};
use fake::{locales::EN, Fake};

use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::{LogExporter, WithExportConfig, WithTonicConfig};
use opentelemetry_sdk::{self, logs::LoggerProvider, runtime};
use rand::Rng;
use tonic::metadata::MetadataMap;
use tracing::{debug, error, info, Level};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::prelude::*;

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

    /// Enable verbose output
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
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
        .with(tracing_subscriber::fmt::layer())
        .with(layer)
        .init();
    Ok(())
}

fn log_demo_data() {
    let mut rng = rand::rng();

    loop {
        let name: String = fake::faker::name::raw::Name(EN).fake();
        if rng.random_bool(0.5) {
            info!("Looking up user: {}", name);
        } else {
            error!("User lookup for name failed: {}", name);
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize tracing subscriber with level based on verbose flag
    let log_level = if args.verbose {
        Level::DEBUG
    } else {
        Level::INFO
    };

    // Configure logging based on the selected output
    match args.log {
        Some(LogOutput::Stdout) => {
            tracing_subscriber::fmt().with_max_level(log_level).init();
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

    // Log startup information
    info!("Starting application");

    // Log feature activation status
    if let Some(log_output) = &args.log {
        info!("Logging enabled with {:?} output", log_output);
    }

    if args.metrics {
        info!("Metrics collection enabled");
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
        error!("No CLI arguments provided");
    }

    Ok(())
}
