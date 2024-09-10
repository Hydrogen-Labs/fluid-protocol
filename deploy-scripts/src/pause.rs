use crate::add_asset::load_core_contracts;
use crate::deploy::deployment::setup_wallet;
use dotenv::dotenv;

use test_utils::interfaces::borrow_operations::borrow_operations_abi;

pub async fn pause_protocol() {
    dotenv().ok();

    let wallet = setup_wallet().await;
    let address = wallet.address();
    println!("🔑 Wallet address: {}", address);

    let core_contracts = load_core_contracts(wallet.clone());

    println!("Are you sure you want to pause the protocol? (y/n)");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    if input.trim().to_lowercase() != "y" {
        println!("Operation cancelled.");
        return;
    }

    let _ = borrow_operations_abi::set_pause_status(&core_contracts.borrow_operations, true)
        .await
        .unwrap();

    println!("Protocol paused successfully");
}

pub async fn unpause_protocol() {
    dotenv().ok();

    let wallet = setup_wallet().await;
    let address = wallet.address();
    println!("🔑 Wallet address: {}", address);

    let core_contracts = load_core_contracts(wallet.clone());

    println!("Are you sure you want to unpause the protocol? (y/n)");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    if input.trim().to_lowercase() != "y" {
        println!("Operation cancelled.");
        return;
    }

    let _ = borrow_operations_abi::set_pause_status(&core_contracts.borrow_operations, false)
        .await
        .unwrap();

    println!("Protocol unpaused successfully");
}
