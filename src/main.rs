use clap::Parser;

mod config;
mod log_runner;
mod tracer;
/// CLI tool for pattern matching
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    file_path: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse command line arguments
    let args = Args::parse();
    let config = config::Config::from_file(&args.file_path)?;
    let log_runner = log_runner::LogRunner::new(config);
    log_runner.run().await?;

    Ok(())
}
