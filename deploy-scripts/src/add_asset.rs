use crate::utils::utils::*;
use dotenv::dotenv;
use fuels::prelude::*;
use fuels::types::{Bits256, U256};
use serde_json::json;

use std::str::FromStr;
use std::{fs::File, io::Write};
use test_utils::data_structures::{AssetContracts, ExistingAssetContracts};
use test_utils::interfaces::oracle::oracle_abi;
use test_utils::interfaces::pyth_oracle::pyth_oracle_abi;
use test_utils::interfaces::redstone_oracle::redstone_oracle_abi;

use test_utils::setup::common::*;
pub async fn add_asset() {
    dotenv().ok();

    let wallet = setup_wallet().await;
    let address: Address = wallet.address().into();
    println!("🔑 Wallet address: {}", address);

    let core_contracts = load_core_contracts(wallet.clone());
    // you will need to set the existing asset contracts here manually and uncomment the below line
    let mut existing_asset_to_initialize: Option<ExistingAssetContracts> =
        Some(ExistingAssetContracts {
            asset: Bech32ContractId::from_str(
                "fuel10skkmgaghljfp8zsxhn780kqptj97lt83rvfterdk5qpuft7ceusku5zl2",
            )
            .unwrap()
            .into(),
            asset_id: AssetId::from_str(
                "4c28edd1de21752e93b347dcda4dfc6931fbf704ed7657f3ba9e759c93422b76",
            )
            .unwrap(),
            pyth_oracle: ContractId::from_str(
                "0xe31e04946c67fb41923f93d50ee7fc1c6c99d6e07c02860c6bea5f4a13919277",
            )
            .unwrap()
            .into(),
            pyth_price_id: Bits256::from_hex_str(
                "0xff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace",
            )
            .unwrap(),
            redstone_oracle: Bech32ContractId::from_str(
                "fuel10m8pzkysfpek9txxqgm653w8jmm9n3qma6029gsux2xxu6a72tysasc8x6",
            )
            .unwrap()
            .into(),
            redstone_price_id: U256::from_dec_str("868587").unwrap(),
            redstone_precision: 9,
            fuel_vm_decimals: 9,
        });
    // existing_asset_to_initialize = None;

    match existing_asset_to_initialize {
        Some(_) => {
            println!("Existing asset to initialize");
        }
        None => {
            println!("Initializing new asset");
        }
    }
    // Deploy the asset contracts
    let asset_contracts = deploy_asset_contracts(&wallet, &existing_asset_to_initialize).await;

    query_oracles(&asset_contracts).await;

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

    write_asset_contracts_to_file(vec![asset_contracts]);

    println!("Asset contracts added successfully");
}

fn write_asset_contracts_to_file(asset_contracts: Vec<AssetContracts<WalletUnlocked>>) {
    // Read existing contracts.json
    let mut contracts: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string("contracts.json").expect("Failed to read contracts.json"),
    )
    .expect("Failed to parse contracts.json");

    // Update asset_contracts field
    contracts["asset_contracts"] =
        json!(asset_contracts.iter().map(|asset_contract| {
        json!({
            "oracle": asset_contract.oracle.contract_id().to_string(),
            "trove_manager": asset_contract.trove_manager.contract_id().to_string(),
            "asset_contract": asset_contract.asset.contract_id().to_string(),
            "asset_id": asset_contract.asset_id.to_string(),
            "pyth_price_id": to_hex_str(&asset_contract.pyth_price_id),
            "pyth_contract": asset_contract.mock_pyth_oracle.contract_id().to_string(),
            "redstone_contract": asset_contract.mock_redstone_oracle.contract_id().to_string(),
            "redstone_price_id": asset_contract.redstone_price_id.to_string(),
            "redstone_precision": asset_contract.redstone_precision,
            "fuel_vm_decimals": asset_contract.fuel_vm_decimals,
        })
    }).collect::<Vec<serde_json::Value>>());

    // Write updated contracts back to file
    let mut file =
        File::create("contracts.json").expect("Failed to open contracts.json for writing");
    file.write_all(serde_json::to_string_pretty(&contracts).unwrap().as_bytes())
        .expect("Failed to write to contracts.json");
}

async fn query_oracles(asset_contracts: &AssetContracts<WalletUnlocked>) {
    let current_pyth_price = pyth_oracle_abi::price(
        &asset_contracts.mock_pyth_oracle,
        &asset_contracts.pyth_price_id,
    )
    .await
    .value;

    let pyth_precision = current_pyth_price.exponent as usize;
    println!(
        "Current pyth price: {:.precision$}",
        current_pyth_price.price as f64 / 10f64.powi(pyth_precision.try_into().unwrap()),
        precision = pyth_precision
    );
    let current_price = oracle_abi::get_price(
        &asset_contracts.oracle,
        &asset_contracts.mock_pyth_oracle,
        &asset_contracts.mock_redstone_oracle,
    )
    .await
    .value;

    println!(
        "Current oracle proxy price: {:.9}",
        current_price as f64 / 1_000_000_000.0
    );

    let current_redstone_price = redstone_oracle_abi::read_prices(
        &asset_contracts.mock_redstone_oracle,
        vec![asset_contracts.redstone_price_id],
    )
    .await
    .value[0]
        .as_u64();

    println!(
        "Current redstone price: {:.9}",
        current_redstone_price as f64 / 1_000_000_000.0
    );
}
pub fn to_hex_str(bits: &Bits256) -> String {
    format!("0x{}", hex::encode(bits.0))
}
