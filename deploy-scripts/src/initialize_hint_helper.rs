use crate::utils::utils::{is_testnet, load_core_contracts, load_hint_helper, setup_wallet};
use dotenv::dotenv;
use test_utils::interfaces::hint_helper::hint_helper_abi;

pub async fn initialize_hint_helper() {
    dotenv().ok();

    let wallet = setup_wallet().await;
    let address = wallet.address();
    println!("🔑 Wallet address: {}", address);

    let is_testnet = is_testnet(wallet.clone()).await;
    let core_contracts = load_core_contracts(wallet.clone(), is_testnet);

    println!("Are you sure you want to initialize the hint helper? (y/n)");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    if input.trim().to_lowercase() != "y" {
        println!("Operation cancelled.");
        return;
    }

    let hint_helper = load_hint_helper(wallet.clone(), is_testnet);

    let _ = hint_helper_abi::initialize(
        &hint_helper,
        core_contracts.sorted_troves.contract.contract_id().into(),
    )
    .await
    .unwrap();

    println!("Hint helper initialized successfully");
}
