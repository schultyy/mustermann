use clap::{Parser, ValueEnum};

use logger::log_demo_data;
use opentelemetry_sdk::{self, logs::LoggerProvider};
use telemetry::{setup_otlp_logger, setup_tracer};
use tracer::simulate_checkout_process;
use tracing::{debug, error, info};
use tracing_subscriber::prelude::*;

mod logger;
mod parser;
mod telemetry;
mod tracer;
mod visitor;

/// CLI tool for pattern matching
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the muster file
    #[arg(short, long)]
    muster_file: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args = Args::parse();
    let logger_provider: Option<LoggerProvider> = None;

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
                        let visitor = visitor::Visitor::new(&muster);
                        visitor.visit_muster();
                    }
                    Err(e) => {
                        error!("Error parsing muster file: {}", e);
                        return Ok(());
                    }
                }
            }
            Err(e) => {
                error!("Error reading muster file: {}", e);
                return Ok(());
            }
        }
    } else if args.muster_file.is_none() {
        error!("No CLI arguments provided. Use --muster-file to specify a muster file or use --log for default behavior.");
    }

    opentelemetry::global::shutdown_tracer_provider();
    if let Some(logger_provider) = logger_provider {
        logger_provider.shutdown()?;
    }
    Ok(())
}
