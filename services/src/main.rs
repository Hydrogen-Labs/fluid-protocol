use dotenv::dotenv;
use std::time::Duration;

mod liquidator;
use liquidator::{Liquidator, LiquidatorConfig};

#[tokio::main]
async fn main() {
    dotenv().ok();

    println!("Starting liquidator service...");

    // Configure the liquidator (optional - will use defaults if not specified)
    let config = LiquidatorConfig {
        check_interval: Duration::from_secs(180), // Check every 3 minutes
        max_troves_per_batch: 5,                  // Process up to 5 troves per batch
    };

    let liquidator = Liquidator::new(Some(config)).await;

    // Start the liquidator service
    match liquidator.start().await {
        Ok(_) => println!("Liquidator service completed successfully"),
        Err(e) => eprintln!("Liquidator service error: {:?}", e),
    }
}
