use crate::constants::{self};
use crate::utils::utils::*;
use dotenv::dotenv;
use fuels::prelude::*;
use fuels::types::Bits256;
use std::str::FromStr;
use test_utils::data_structures::{AssetConfig, ExistingAssetContracts, PythConfig};
use test_utils::setup::common::*;

pub async fn add_asset(symbol: &str) {
    dotenv().ok();

    let wallet = setup_wallet().await;
    let network_name = wallet.provider().unwrap().chain_info().await.unwrap().name;
    let is_testnet = is_testnet(wallet.clone()).await;
    let address: Address = wallet.address().into();
    println!("🔑 Wallet address: {}", address);
    println!("Network name: {}", network_name);
    println!("Is testnet: {}", is_testnet);

    let core_contracts = load_core_contracts(wallet.clone(), is_testnet);

    // Get asset constants based on symbol and network type
    let asset_constants = match (symbol.to_uppercase().as_str(), is_testnet) {
        // Testnet
        ("FUEL", true) => &constants::TESTNET_FUEL_CONSTANTS,
        ("ETH", true) => &constants::TESTNET_ETH_CONSTANTS,
        ("WSTETH", true) => &constants::TESTNET_WSTETH_CONSTANTS,
        ("EZETH", true) => &constants::TESTNET_EZETH_CONSTANTS,
        ("WEETH", true) => &constants::TESTNET_WEETH_CONSTANTS,
        ("RSETH", true) => &constants::TESTNET_RSETH_CONSTANTS,
        ("METH", true) => &constants::TESTNET_METH_CONSTANTS,
        // Mainnet
        ("FUEL", false) => &constants::MAINNET_FUEL_CONSTANTS,
        ("ETH", false) => &constants::MAINNET_ETH_CONSTANTS,
        ("WSTETH", false) => &constants::MAINNET_WSTETH_CONSTANTS,
        ("EZETH", false) => &constants::MAINNET_EZETH_CONSTANTS,
        ("WEETH", false) => &constants::MAINNET_WEETH_CONSTANTS,
        ("RSETH", false) => &constants::MAINNET_RSETH_CONSTANTS,
        ("METH", false) => &constants::MAINNET_METH_CONSTANTS,
        // Add more assets as needed
        (symbol, is_testnet) => panic!(
            "Unsupported asset symbol '{}' for {} network",
            symbol,
            if is_testnet { "testnet" } else { "mainnet" }
        ),
    };

    let existing_asset_to_initialize: ExistingAssetContracts = ExistingAssetContracts {
        symbol: symbol.to_string(),
        asset: match is_testnet {
            true => None,
            false => Some(AssetConfig {
                asset: ContractId::from_str(asset_constants.asset_contract_id.unwrap()).unwrap(),
                asset_id: AssetId::from_str(asset_constants.asset_id.unwrap()).unwrap(),
                fuel_vm_decimals: asset_constants.decimals,
            }),
        },
        pyth_oracle: Some(PythConfig {
            contract: ContractId::from_str(asset_constants.pyth_contract_id).unwrap(),
            price_id: Bits256::from_hex_str(asset_constants.pyth_price_id).unwrap(),
        }),
        redstone_oracle: None, // TODO: Add redstone oracle when it's ready
    };

    // Redstone oracle is not required for initialization
    if existing_asset_to_initialize.asset.is_none()
        || existing_asset_to_initialize.pyth_oracle.is_none()
    {
        // If testnet then cause a failure so it's obvious
        if !is_testnet {
            panic!("Mainnet assets must have an asset and pyth oracle");
        }

        println!("Initializing new asset");
    } else {
        println!("Existing asset to register");
    }

    // Deploy the asset contracts
    let asset_contracts =
        deploy_asset_contracts(&wallet, &existing_asset_to_initialize, false, false).await;

    query_oracles(&asset_contracts, wallet.clone()).await;

    println!("Are you sure you want to initialize the asset? (y/n)");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line");
    if input.trim().to_lowercase() != "y" {
        println!("Operation cancelled.");
        return;
    }

    initialize_asset(&core_contracts, &asset_contracts)
        .await
        .unwrap();

    write_asset_contracts_to_file(vec![asset_contracts], is_testnet);

    println!("Asset contracts added successfully");
}
