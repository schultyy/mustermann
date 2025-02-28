use ctrlc;
use fake::{locales::EN, Fake};
use rand::Rng;
use tracing::{error, info};

pub fn log_demo_data() {
    let mut rng = rand::rng();

    // Create a channel to listen for Ctrl+C
    let (tx, rx) = std::sync::mpsc::channel();

    // Set up Ctrl+C handler
    ctrlc::set_handler(move || {
        tx.send(()).expect("Could not send signal on channel");
    })
    .expect("Error setting Ctrl-C handler");

    loop {
        // Check if Ctrl+C was pressed
        if rx.try_recv().is_ok() {
            info!("Received interrupt signal, shutting down");
            break;
        }

        let name: String = fake::faker::name::raw::Name(EN).fake();
        if rng.random_bool(0.5) {
            info!("Looking up user: {}", name);
        } else {
            error!("User lookup for name failed: {}", name);
        }

        // Add a small delay to prevent CPU hogging
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
