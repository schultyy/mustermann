use clap::Parser;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod config;
mod log_runner;
mod otel;

/// CLI tool for pattern matching
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    file_path: String,
    otel_endpoint: Option<String>,
    /// The name of the service to be used in the logs. Defaults to "mustermann"
    #[arg(short, long, default_value = "mustermann")]
    service_name: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args = Args::parse();
    if let Some(otel_endpoint) = args.otel_endpoint {
        otel::setup_otlp_logger(&otel_endpoint, &args.service_name)?;
    } else {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "info".into()),
            )
            .with(tracing_subscriber::fmt::layer())
            .init();
    }
    let config = config::Config::from_file(&args.file_path)?;
    let log_runner = log_runner::LogRunner::new(config);
    log_runner.run().await?;

    Ok(())
}
