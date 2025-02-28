use fake::{faker::company::en::*, faker::name::en::*, Fake};
use rand::Rng;
use std::{thread, time::Duration};
use tracing::{info, info_span, instrument, warn};

/// Simulates a complete checkout process in a store with multiple spans
pub fn simulate_checkout_process() {
    let customer_name: String = Name().fake();
    let store_name: String = CompanyName().fake();

    // Create the main checkout span
    let checkout_span = info_span!("checkout_process",
        customer.name = %customer_name,
        store.name = %store_name
    );

    // Execute the entire checkout process within the main span
    checkout_span.in_scope(|| {
        info!(
            "Customer {} started checkout at {}",
            customer_name, store_name
        );

        // Simulate scanning items
        let items = scan_items();

        // Process payment
        let payment_successful = process_payment(&customer_name, calculate_total(&items));

        // Generate receipt if payment was successful
        if payment_successful {
            generate_receipt(&customer_name, &items);
            info!("Checkout completed successfully for {}", customer_name);
        } else {
            warn!(
                "Checkout failed for {}: payment unsuccessful",
                customer_name
            );
        }
    });
}

/// Simulates scanning items at checkout
#[instrument(name = "scan_items", skip_all)]
fn scan_items() -> Vec<(String, f64)> {
    let mut rng = rand::rng();
    let item_count = rng.random_range(1..6);

    info!("Scanning {} items", item_count);

    let possible_items = [
        ("Milk", 3.99),
        ("Bread", 2.49),
        ("Eggs", 4.99),
        ("Coffee", 8.99),
        ("Cheese", 5.49),
        ("Apples", 3.29),
        ("Cereal", 4.79),
        ("Chicken", 9.99),
        ("Pasta", 1.99),
        ("Chocolate", 2.99),
    ];

    let mut items = Vec::new();

    for i in 0..item_count {
        let item_index = rng.random_range(0..possible_items.len());
        let (name, price) = possible_items[item_index];

        // Create a span for each item scan
        let scan_span = info_span!(
            "scan_item",
            item.name = name,
            item.price = price,
            item.index = i
        );

        // Execute the item scanning within its own span
        scan_span.in_scope(|| {
            // Simulate scanning delay
            let scan_time = rng.random_range(100..500);
            thread::sleep(Duration::from_millis(scan_time));

            info!("Scanned item: {} (${:.2})", name, price);

            // Occasionally simulate a scanning issue
            if rng.random_bool(0.1) {
                warn!("Scanning issue detected with {}, retrying...", name);
                thread::sleep(Duration::from_millis(300));
                info!("Rescan successful");
            }
        });

        items.push((name.to_string(), price));
    }

    items
}

/// Calculates the total price of all items
#[instrument(name = "calculate_total", skip_all, fields(item_count))]
fn calculate_total(items: &[(String, f64)]) -> f64 {
    let span = tracing::Span::current();
    span.record("item_count", items.len());

    let mut total = 0.0;
    for (name, price) in items {
        info!("Adding {} (${:.2}) to total", name, price);
        total += price;
    }

    // Apply random discount occasionally
    let mut rng = rand::rng();
    if rng.random_bool(0.3) {
        let discount_percent = rng.random_range(5..20);
        let discount_amount = total * (discount_percent as f64 / 100.0);
        total -= discount_amount;
        info!(
            "Applied {}% discount: -${:.2}",
            discount_percent, discount_amount
        );
    }

    // Add tax
    let tax_rate = 0.08;
    let tax_amount = total * tax_rate;
    total += tax_amount;
    info!("Added tax (8%): ${:.2}", tax_amount);

    info!("Final total: ${:.2}", total);
    total
}

/// Simulates payment processing
#[instrument(name = "process_payment", skip_all, fields(amount, customer.name))]
fn process_payment(customer_name: &str, amount: f64) -> bool {
    let span = tracing::Span::current();
    span.record("amount", amount);
    span.record("customer.name", customer_name);

    info!("Processing payment of ${:.2} for {}", amount, customer_name);

    // Simulate payment processing time
    let mut rng = rand::rng();
    let processing_time = rng.random_range(500..2000);
    thread::sleep(Duration::from_millis(processing_time));

    // Simulate payment methods
    let payment_methods = ["Credit Card", "Debit Card", "Mobile Payment", "Cash"];
    let payment_method = payment_methods[rng.random_range(0..payment_methods.len())];

    info!("Payment method: {}", payment_method);

    // Simulate occasional payment failures
    let success = rng.random_bool(0.9);

    if success {
        info!("Payment approved for ${:.2}", amount);
    } else {
        warn!("Payment declined for ${:.2}", amount);
    }

    success
}

/// Generates a receipt for the purchase
#[instrument(name = "generate_receipt", skip_all)]
fn generate_receipt(customer_name: &str, items: &[(String, f64)]) {
    info!("Generating receipt for {}", customer_name);

    // Simulate receipt generation time
    thread::sleep(Duration::from_millis(300));

    info!("Receipt details:");
    for (i, (name, price)) in items.iter().enumerate() {
        info!("  {}. {} - ${:.2}", i + 1, name, price);
    }

    let total: f64 = items.iter().map(|(_, price)| price).sum();
    info!("  Total: ${:.2}", total);

    // Simulate printing delay
    thread::sleep(Duration::from_millis(500));

    info!("Receipt printed successfully");
}

/// Runs a continuous simulation of checkout processes
pub fn run_checkout_simulation() {
    info!("Starting checkout simulation. Press Ctrl+C to stop.");

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
            info!("Received interrupt signal, shutting down checkout simulation");
            break;
        }

        simulate_checkout_process();

        // Wait between checkout simulations
        let mut rng = rand::rng();
        let wait_time = rng.random_range(1000..3000);
        thread::sleep(Duration::from_millis(wait_time));
    }
}
