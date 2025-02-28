use tracing::{info, Level};

fn main() {
    // Initialize tracing subscriber
    tracing_subscriber::fmt()
        .with_max_level(Level::INFO)
        .init();

    info!("Starting application");
    println!("Hello, world!");
    info!("Application finished");
}
