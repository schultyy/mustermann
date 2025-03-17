use clap::Parser;

use opentelemetry_sdk::{self, logs::LoggerProvider};
use tracing::{error, info};
use tracing_subscriber::EnvFilter;

mod parser;
mod telemetry;
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

    // Initialize tracing with appropriate filter
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .init();

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

                        // Create and run the visitor
                        let visitor = visitor::Visitor::new(&muster);
                        match visitor.run().await {
                            Ok(_) => info!("Visitor completed successfully"),
                            Err(e) => error!("Visitor failed: {:?}", e),
                        }
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
