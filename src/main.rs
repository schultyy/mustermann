use clap::{Parser, ValueEnum};
use fake::{locales::EN, Fake};
use tracing::{info, debug, Level};

/// CLI tool for pattern matching
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
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

fn main() {
    // Parse command line arguments
    let args = Args::parse();

    // Initialize tracing subscriber with level based on verbose flag
    let log_level = if args.verbose { Level::DEBUG } else { Level::INFO };
    
    // Configure logging based on the selected output
    match args.log {
        Some(LogOutput::Stdout) => {
            tracing_subscriber::fmt()
                .with_max_level(log_level)
                .init();
            info!("Logging initialized with stdout output");
        }
        Some(LogOutput::Otlp) => {
            // This is a stub for now
            tracing_subscriber::fmt()
                .with_max_level(log_level)
                .init();
            info!("OTLP logging would be initialized here (stub)");
        }
        None => {
            // Initialize minimal logging for internal use
            tracing_subscriber::fmt()
                .with_max_level(Level::WARN)
                .init();
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
        
        // Demo log data
        loop {
            let name : String = fake::faker::name::raw::Name(EN).fake();
            info!("Looking up user: {}", name);
        }
    }
    
    println!("Hello, world!");
    info!("Application finished");
}
