use clap::Parser;
use tracing::{error, info};

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

    /// OTLP endpoint for exporting logs
    #[arg(long)]
    otlp_endpoint: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize combined logging for both internal telemetry and muster data
    let logger_provider = match &args.otlp_endpoint {
        Some(endpoint) => {
            // This info won't be seen because logging isn't set up yet, but that's okay
            let provider = telemetry::setup_combined_logging(Some(endpoint))?;
            info!("Logging initialized with OTLP export to: {}", endpoint);
            provider
        }
        None => {
            let provider = telemetry::setup_combined_logging(None)?;
            info!("Logging initialized (local only, no OTLP export)");
            provider
        }
    };

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

    // Shutdown providers
    info!("Shutting down...");
    opentelemetry::global::shutdown_tracer_provider();
    logger_provider.shutdown()?;

    Ok(())
}
