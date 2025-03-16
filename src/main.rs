use clap::{Parser, ValueEnum};

use logger::log_demo_data;
use opentelemetry_sdk::{self, logs::LoggerProvider};
use telemetry::{setup_otlp_logger, setup_tracer};
use tracer::simulate_checkout_process;
use tracing::{debug, info};
use tracing_subscriber::prelude::*;

mod logger;
mod parser;
mod telemetry;
mod tracer;

/// CLI tool for pattern matching
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the muster file
    #[arg(short, long)]
    muster_file: Option<String>,

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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args = Args::parse();
    let mut logger_provider: Option<LoggerProvider> = None;

    // Parse muster file if provided
    if let Some(muster_file) = &args.muster_file {
        info!("Parsing muster file: {}", muster_file);
        match std::fs::read_to_string(muster_file) {
            Ok(content) => {
                match parser::parser::parse_muster(&content) {
                    Ok(muster) => {
                        info!(
                            "Successfully parsed muster with {} logs block(s)",
                            muster.logs_blocks.len()
                        );
                        // In a future implementation, this is where we would process the muster file
                        // and generate logs based on the parsed AST
                    }
                    Err(e) => {
                        eprintln!("Error parsing muster file: {}", e);
                        return Ok(());
                    }
                }
            }
            Err(e) => {
                eprintln!("Error reading muster file: {}", e);
                return Ok(());
            }
        }
    }

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
    } else if args.muster_file.is_none() {
        eprintln!("No CLI arguments provided. Use --muster-file to specify a muster file or use --log for default behavior.");
    }

    opentelemetry::global::shutdown_tracer_provider();
    if let Some(logger_provider) = logger_provider {
        logger_provider.shutdown()?;
    }
    Ok(())
}
