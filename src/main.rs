use clap::Parser;

mod config;
mod logger;
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
    println!("{:?}", config);

    Ok(())
}
