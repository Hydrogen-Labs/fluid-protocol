use crate::{
    constants::{MAINNET_CONTRACTS_FILE, TESTNET_CONTRACTS_FILE},
    utils::utils::{is_testnet, load_core_contracts, setup_wallet},
};
use dotenv::dotenv;
use pbr::ProgressBar;
use serde_json::json;
use std::fs::File;
use std::io::Write;
use test_utils::{
    interfaces::proxy::{proxy_abi, Proxy},
    setup::common::{
        deploy_borrow_operations, deploy_protocol_manager, deploy_trove_manager_contract,
    },
};

pub async fn migrate_to_v2() {
    dotenv().ok();

    let wallet = setup_wallet().await;
    let address = wallet.address().hash().to_string();
    println!("🔑 Wallet address: 0x{}", address);

    let is_testnet = is_testnet(wallet.clone()).await;
    let core_contracts = load_core_contracts(wallet.clone(), is_testnet);

    println!(
        "Network: {}",
        match is_testnet {
            true => "Testnet",
            false => "Mainnet",
        }
    );

    println!("Are you sure you want to migrate contracts to v2? (y/n)");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    if input.trim().to_lowercase() != "y" {
        println!("Operation cancelled.");
        return;
    }

    // Deploy new versions
    println!("Deploying new contract versions...");
    let new_borrow_operations = deploy_borrow_operations(&wallet).await;
    let new_protocol_manager = deploy_protocol_manager(&wallet).await;
    let new_trove_manager = deploy_trove_manager_contract(&wallet).await;

    let length = core_contracts.asset_contracts.len() + 2;
    let mut pb = ProgressBar::new(length as u64);

    // Update proxies
    println!("Updating proxy targets...");

    // Update BorrowOperations proxy
    let borrow_operations_proxy = Proxy::new(
        core_contracts
            .borrow_operations
            .contract
            .contract_id()
            .clone(),
        wallet.clone(),
    );
    let _ = proxy_abi::set_proxy_target(
        &borrow_operations_proxy,
        new_borrow_operations.implementation_id.into(),
    )
    .await
    .unwrap();
    pb.inc();
    // Update ProtocolManager proxy
    let protocol_manager_proxy = Proxy::new(
        core_contracts
            .protocol_manager
            .contract
            .contract_id()
            .clone(),
        wallet.clone(),
    );
    let _ = proxy_abi::set_proxy_target(
        &protocol_manager_proxy,
        new_protocol_manager.implementation_id.into(),
    )
    .await
    .unwrap();
    pb.inc();

    // Update all TroveManager proxies to point to the same new implementation
    for asset_contract in &core_contracts.asset_contracts {
        let trove_manager_proxy = Proxy::new(
            asset_contract.trove_manager.contract.contract_id().clone(),
            wallet.clone(),
        );
        let _ = proxy_abi::set_proxy_target(
            &trove_manager_proxy,
            new_trove_manager.implementation_id.into(),
        )
        .await
        .unwrap();
        pb.inc();
    }

    // Write the updated contracts to file
    let json = std::fs::read_to_string(match is_testnet {
        true => TESTNET_CONTRACTS_FILE,
        false => MAINNET_CONTRACTS_FILE,
    })
    .unwrap();
    let mut contracts: serde_json::Value = serde_json::from_str(&json).unwrap();

    // Update implementation IDs in the JSON
    contracts["borrow_operations_implementation_id"] =
        json!(format!("0x{}", new_borrow_operations.implementation_id));
    contracts["protocol_manager_implementation_id"] =
        json!(format!("0x{}", new_protocol_manager.implementation_id));

    // Update all trove manager implementation IDs in asset_contracts
    let asset_contracts = contracts["asset_contracts"].as_array_mut().unwrap();
    for asset_contract in asset_contracts {
        asset_contract["trove_manager_implementation_id"] =
            json!(format!("0x{}", new_trove_manager.implementation_id));
    }

    // Write updated contracts back to file
    let mut file = File::create(match is_testnet {
        true => TESTNET_CONTRACTS_FILE,
        false => MAINNET_CONTRACTS_FILE,
    })
    .unwrap();
    file.write_all(serde_json::to_string_pretty(&contracts).unwrap().as_bytes())
        .unwrap();

    println!("Migration completed successfully!");
    println!("New contract addresses:");
    println!(
        "BorrowOperations: {}",
        new_borrow_operations.implementation_id
    );
    println!(
        "ProtocolManager: {}",
        new_protocol_manager.implementation_id
    );
    println!("TroveManager: {}", new_trove_manager.implementation_id);

    pb.finish();
}
