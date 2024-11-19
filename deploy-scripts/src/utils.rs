pub mod utils {
    use csv::ReaderBuilder;
    use fuels::accounts::{provider::Provider, wallet::WalletUnlocked};
    use fuels::prelude::*;
    use fuels::types::bech32::Bech32ContractId;
    use fuels::types::{Bits256, Identity, U256};
    use serde_json::json;
    use std::fs::File;
    use std::io::Write;
    use std::str::FromStr;
    use test_utils::data_structures::{
        AssetContracts, AssetContractsOptionalRedstone, ContractInstance, PRECISION,
    };
    use test_utils::interfaces::multi_trove_getter::MultiTroveGetter;
    use test_utils::interfaces::oracle::oracle_abi;
    use test_utils::interfaces::pyth_oracle::pyth_oracle_abi;
    use test_utils::interfaces::redstone_oracle::redstone_oracle_abi;
    use test_utils::interfaces::vesting::{VestingSchedule, TOTAL_AMOUNT_VESTED};
    use test_utils::setup::common::get_absolute_path_from_relative;
    use test_utils::{
        data_structures::ProtocolContracts,
        interfaces::{
            active_pool::ActivePool, borrow_operations::BorrowOperations,
            coll_surplus_pool::CollSurplusPool, community_issuance::CommunityIssuance,
            default_pool::DefaultPool, fpt_staking::FPTStaking, fpt_token::FPTToken,
            protocol_manager::ProtocolManager, pyth_oracle::PythCore,
            redstone_oracle::RedstoneCore, sorted_troves::SortedTroves,
            stability_pool::StabilityPool, token::Token, trove_manager::TroveManagerContract,
            vesting::VestingContract,
        },
    };

    use crate::constants::{
        MAINNET_CONTRACTS_FILE, MAINNET_RPC, TESTNET_CONTRACTS_FILE, TESTNET_RPC,
    };
    pub async fn setup_wallet() -> WalletUnlocked {
        let network =
            std::env::var("NETWORK").expect("NETWORK must be set to 'mainnet' or 'testnet'");

        let rpc: String = match network.as_str() {
            "mainnet" => MAINNET_RPC.to_string(),
            "testnet" => TESTNET_RPC.to_string(),
            _ => panic!("❌ NETWORK must be 'mainnet' or 'testnet'"),
        };

        let provider = match Provider::connect(rpc).await {
            Ok(p) => p,
            Err(error) => panic!("❌ Problem creating provider: {:#?}", error),
        };

        let secret = match std::env::var("SECRET") {
            Ok(s) => s,
            Err(error) => panic!("❌ Cannot find .env file: {:#?}", error),
        };

        WalletUnlocked::new_from_mnemonic_phrase_with_path(
            &secret,
            Some(provider),
            "m/44'/1179993420'/0'/0/0",
        )
        .unwrap()
    }

    pub fn load_core_contracts(
        wallet: WalletUnlocked,
        is_testnet: bool,
    ) -> ProtocolContracts<WalletUnlocked> {
        let json = std::fs::read_to_string(match is_testnet {
            true => TESTNET_CONTRACTS_FILE,
            false => MAINNET_CONTRACTS_FILE,
        })
        .unwrap();
        let contracts: serde_json::Value = serde_json::from_str(&json).unwrap();

        let borrow_operations_contract_id: Bech32ContractId = contracts["borrow_operations"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let borrow_operations_implementation_id: ContractId = contracts
            ["borrow_operations_implementation_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let borrow_operations = ContractInstance::new(
            BorrowOperations::new(borrow_operations_contract_id.clone(), wallet.clone()),
            borrow_operations_implementation_id.into(),
        );
        let usdf_contract_id: Bech32ContractId =
            contracts["usdf"].as_str().unwrap().parse().unwrap();
        let usdf_implementation_id: ContractId = contracts["usdf_implementation_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let usdf = ContractInstance::new(
            test_utils::interfaces::usdf_token::USDFToken::new(usdf_contract_id, wallet.clone()),
            usdf_implementation_id.into(),
        );

        let stability_pool_contract_id: Bech32ContractId = contracts["stability_pool"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let stability_pool_implementation_id: ContractId = contracts
            ["stability_pool_implementation_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let stability_pool = ContractInstance::new(
            StabilityPool::new(stability_pool_contract_id.clone(), wallet.clone()),
            stability_pool_implementation_id.into(),
        );

        let protocol_manager_contract_id: Bech32ContractId = contracts["protocol_manager"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let protocol_manager_implementation_id: ContractId = contracts
            ["protocol_manager_implementation_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let protocol_manager = ContractInstance::new(
            ProtocolManager::new(protocol_manager_contract_id.clone(), wallet.clone()),
            protocol_manager_implementation_id.into(),
        );

        let fpt_staking_contract_id: Bech32ContractId =
            contracts["fpt_staking"].as_str().unwrap().parse().unwrap();
        let fpt_staking_implementation_id: ContractId = contracts["fpt_staking_implementation_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let fpt_staking = ContractInstance::new(
            FPTStaking::new(fpt_staking_contract_id.clone(), wallet.clone()),
            fpt_staking_implementation_id.into(),
        );

        let fpt_token_contract_id: Bech32ContractId =
            contracts["fpt_token"].as_str().unwrap().parse().unwrap();
        let fpt_token_implementation_id: ContractId = contracts["fpt_token_implementation_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let fpt_token = ContractInstance::new(
            FPTToken::new(fpt_token_contract_id.clone(), wallet.clone()),
            fpt_token_implementation_id.into(),
        );

        let community_issuance_contract_id: Bech32ContractId = contracts["community_issuance"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let community_issuance_implementation_id: ContractId = contracts
            ["community_issuance_implementation_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let community_issuance = ContractInstance::new(
            CommunityIssuance::new(community_issuance_contract_id.clone(), wallet.clone()),
            community_issuance_implementation_id.into(),
        );

        let coll_surplus_pool_contract_id: Bech32ContractId = contracts["coll_surplus_pool"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let coll_surplus_pool_implementation_id: ContractId = contracts
            ["coll_surplus_pool_implementation_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let coll_surplus_pool = ContractInstance::new(
            CollSurplusPool::new(coll_surplus_pool_contract_id.clone(), wallet.clone()),
            coll_surplus_pool_implementation_id.into(),
        );

        let default_pool_contract_id: Bech32ContractId =
            contracts["default_pool"].as_str().unwrap().parse().unwrap();
        let default_pool_implementation_id: ContractId = contracts
            ["default_pool_implementation_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let default_pool = ContractInstance::new(
            DefaultPool::new(default_pool_contract_id.clone(), wallet.clone()),
            default_pool_implementation_id.into(),
        );

        let active_pool_contract_id: Bech32ContractId =
            contracts["active_pool"].as_str().unwrap().parse().unwrap();
        let active_pool_implementation_id: ContractId = contracts["active_pool_implementation_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let active_pool = ContractInstance::new(
            ActivePool::new(active_pool_contract_id.clone(), wallet.clone()),
            active_pool_implementation_id.into(),
        );

        let sorted_troves_contract_id: Bech32ContractId = contracts["sorted_troves"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let sorted_troves_implementation_id: ContractId = contracts
            ["sorted_troves_implementation_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let sorted_troves = ContractInstance::new(
            SortedTroves::new(sorted_troves_contract_id.clone(), wallet.clone()),
            sorted_troves_implementation_id.into(),
        );

        let vesting_contract_id: Bech32ContractId = contracts["vesting_contract"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let vesting_contract_implementation_id: ContractId = contracts
            ["vesting_contract_implementation_id"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let vesting_contract = ContractInstance::new(
            VestingContract::new(vesting_contract_id.clone(), wallet.clone()),
            vesting_contract_implementation_id.into(),
        );

        let fpt_asset_id: AssetId =
            AssetId::from_str(contracts["fpt_asset_id"].as_str().unwrap()).unwrap();

        let usdf_asset_id: AssetId =
            AssetId::from_str(contracts["usdf_asset_id"].as_str().unwrap()).unwrap();

        let asset_contracts = contracts["asset_contracts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|asset_contract| {
                let asset_contract_id: Bech32ContractId = asset_contract["asset_contract"]
                    .as_str()
                    .unwrap()
                    .parse()
                    .unwrap();
                let asset_id =
                    AssetId::from_str(asset_contract["asset_id"].as_str().unwrap()).unwrap();
                let oracle_contract_id: Bech32ContractId =
                    asset_contract["oracle"].as_str().unwrap().parse().unwrap();
                let oracle_implementation_id: ContractId = asset_contract
                    ["oracle_implementation_id"]
                    .as_str()
                    .unwrap()
                    .parse()
                    .unwrap();
                let trove_manager_contract_id: Bech32ContractId = asset_contract["trove_manager"]
                    .as_str()
                    .unwrap()
                    .parse()
                    .unwrap();
                let trove_manager_implementation_id: ContractId = asset_contract
                    ["trove_manager_implementation_id"]
                    .as_str()
                    .unwrap()
                    .parse()
                    .unwrap();
                let pyth_contract_id: Bech32ContractId = asset_contract["pyth_contract"]
                    .as_str()
                    .unwrap()
                    .parse()
                    .unwrap();
                let redstone_contract_id: Bech32ContractId = asset_contract["redstone_contract"]
                    .as_str()
                    .unwrap_or("0")
                    .parse()
                    .unwrap_or(Bech32ContractId::default());

                AssetContracts {
                    asset: Token::new(asset_contract_id, wallet.clone()),
                    asset_id,
                    oracle: ContractInstance::new(
                        test_utils::interfaces::oracle::Oracle::new(
                            oracle_contract_id.clone(),
                            wallet.clone(),
                        ),
                        oracle_implementation_id.into(),
                    ),
                    trove_manager: ContractInstance::new(
                        TroveManagerContract::new(
                            trove_manager_contract_id.clone(),
                            wallet.clone(),
                        ),
                        trove_manager_implementation_id.into(),
                    ),
                    fuel_vm_decimals: asset_contract["fuel_vm_decimals"].as_u64().unwrap() as u32,
                    mock_pyth_oracle: PythCore::new(pyth_contract_id, wallet.clone()),
                    mock_redstone_oracle: RedstoneCore::new(redstone_contract_id, wallet.clone()),
                    pyth_price_id: Bits256::from_hex_str(
                        asset_contract["pyth_price_id"].as_str().unwrap(),
                    )
                    .unwrap(),
                    redstone_precision: asset_contract["redstone_precision"].as_u64().unwrap_or(9)
                        as u32,
                    redstone_price_id: U256::from_str(
                        asset_contract["redstone_price_id"].as_str().unwrap_or("0"),
                    )
                    .unwrap(),
                    symbol: asset_contract["symbol"].as_str().unwrap().to_string(),
                }
            })
            .collect();

        let protocol_contracts = ProtocolContracts {
            borrow_operations,
            usdf,
            stability_pool,
            protocol_manager,
            asset_contracts,
            fpt_staking,
            fpt_token,
            fpt_asset_id,
            usdf_asset_id,
            community_issuance,
            vesting_contract,
            coll_surplus_pool,
            sorted_troves,
            default_pool,
            active_pool,
        };

        protocol_contracts
    }

    pub fn load_multi_trove_getter(
        wallet: WalletUnlocked,
        is_testnet: bool,
    ) -> MultiTroveGetter<WalletUnlocked> {
        let json = std::fs::read_to_string(match is_testnet {
            true => TESTNET_CONTRACTS_FILE,
            false => MAINNET_CONTRACTS_FILE,
        })
        .unwrap();
        let contracts: serde_json::Value = serde_json::from_str(&json).unwrap();
        MultiTroveGetter::new(
            contracts["multi_trove_getter"]
                .as_str()
                .unwrap()
                .parse::<Bech32ContractId>()
                .unwrap(),
            wallet,
        )
    }

    pub async fn is_testnet(wallet: WalletUnlocked) -> bool {
        let network_name = wallet.provider().unwrap().chain_info().await.unwrap().name;
        network_name.to_lowercase().contains("testnet")
    }

    pub fn write_asset_contracts_to_file(
        asset_contracts: Vec<AssetContractsOptionalRedstone<WalletUnlocked>>,
        is_testnet: bool,
    ) {
        // Read existing contracts.json
        let mut contracts: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(match is_testnet {
                true => TESTNET_CONTRACTS_FILE,
                false => MAINNET_CONTRACTS_FILE,
            })
            .expect("Failed to read contracts.json"),
        )
        .expect("Failed to parse contracts.json");

        // Get existing asset_contracts or create an empty array if it doesn't exist
        let mut existing_asset_contracts = contracts["asset_contracts"]
            .as_array()
            .cloned()
            .unwrap_or_else(Vec::new);

        // Add new asset_contracts to the existing ones
        for asset_contract in asset_contracts {
            existing_asset_contracts.push(json!({
                "symbol": asset_contract.symbol,
                "oracle": asset_contract.oracle.contract.contract_id().to_string(),
                "oracle_implementation_id": format!("0x{}", asset_contract.oracle.implementation_id.to_string()),
                "trove_manager": asset_contract.trove_manager.contract.contract_id().to_string(),
                "trove_manager_implementation_id": format!("0x{}", asset_contract.trove_manager.implementation_id.to_string()),
                "asset_contract": asset_contract.asset.contract_id().to_string(),
                "asset_id": format!("0x{}", asset_contract.asset_id.to_string()),
                "pyth_price_id": to_hex_str(&asset_contract.pyth_price_id),
                "pyth_contract": asset_contract.mock_pyth_oracle.contract_id().to_string(),
                "redstone": match &asset_contract.redstone_config {
                    Some(redstone_config) => {
                        json!({
                            "redstone_contract": redstone_config.contract.to_string(),
                            "redstone_price_id": redstone_config.price_id.to_string(),
                            "redstone_precision": redstone_config.precision,
                        })
                    },
                    None => json!(null),
                },
                "fuel_vm_decimals": asset_contract.fuel_vm_decimals,
            }));
        }

        // Update asset_contracts field with the combined list
        contracts["asset_contracts"] = json!(existing_asset_contracts);

        // Write updated contracts back to file
        let mut file = File::create(match is_testnet {
            true => TESTNET_CONTRACTS_FILE,
            false => MAINNET_CONTRACTS_FILE,
        })
        .expect("Failed to open contracts.json for writing");
        file.write_all(serde_json::to_string_pretty(&contracts).unwrap().as_bytes())
            .expect("Failed to write to contracts.json");
    }

    pub async fn query_oracles(
        asset_contracts: &AssetContractsOptionalRedstone<WalletUnlocked>,
        wallet: WalletUnlocked,
    ) {
        let current_pyth_price = pyth_oracle_abi::price_unsafe(
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
        let mut redstone_contract: Option<RedstoneCore<WalletUnlocked>> = None;
        match &asset_contracts.redstone_config {
            Some(redstone_config) => {
                redstone_contract =
                    Some(RedstoneCore::new(redstone_config.contract, wallet.clone()));
            }
            None => {}
        }

        let current_price = oracle_abi::get_price(
            &asset_contracts.oracle,
            &asset_contracts.mock_pyth_oracle,
            &redstone_contract,
        )
        .await
        .unwrap()
        .value;

        println!(
            "Current oracle proxy price: {:.9}",
            current_price as f64 / 1_000_000_000.0
        );
        match &asset_contracts.redstone_config {
            Some(redstone_config) => {
                let redstone_precision = redstone_config.precision as usize;
                let current_redstone_price = redstone_oracle_abi::read_prices(
                    &redstone_contract.unwrap(),
                    vec![redstone_config.price_id],
                )
                .await
                .value[0]
                    .as_u64();

                println!(
                    "Current redstone price: {:.precision$}",
                    current_redstone_price as f64
                        / 10f64.powi(redstone_precision.try_into().unwrap()),
                    precision = redstone_precision
                );
            }
            None => {}
        }
    }
    pub fn to_hex_str(bits: &Bits256) -> String {
        format!("0x{}", hex::encode(bits.0))
    }

    pub fn load_vesting_schedules_from_csv(
        path: &str,
        cliff_percentage: f64,
        seconds_to_cliff: u64,
        seconds_vesting_duration: u64,
        treasury_identity: Identity,
    ) -> Vec<VestingSchedule> {
        let absolute_path = get_absolute_path_from_relative(path);
        let file = File::open(&absolute_path).expect("Failed to open file");
        let mut reader = ReaderBuilder::new()
            .flexible(true)
            .trim(csv::Trim::All)
            .has_headers(false)
            .from_reader(file);

        let now_unix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("Time went backwards")
            .as_secs();

        let cliff_timestamp = now_unix + seconds_to_cliff;
        let end_timestamp = cliff_timestamp + seconds_vesting_duration;
        let now_unix_and_5_minutes = now_unix + 5 * 60;

        let now_and_5_minutes = tai64::Tai64::from_unix(now_unix_and_5_minutes.try_into().unwrap());
        let cliff_timestamp = tai64::Tai64::from_unix(cliff_timestamp.try_into().unwrap());
        let end_timestamp = tai64::Tai64::from_unix(end_timestamp.try_into().unwrap());

        let mut schedules = Vec::new();

        for result in reader.records() {
            let record = result.expect("Failed to read CSV record");
            if record.len() < 5 || record[1].is_empty() {
                panic!("Invalid row found in CSV: {:?}", record);
            }

            // println!("record: {:?}", record);

            let total_amount = (record[1].replace([',', '"'], "").parse::<f64>().unwrap()
                * PRECISION as f64) as u64;
            let recipient = if !record[2].is_empty() {
                Identity::Address(Address::from_str(&record[2]).unwrap())
            } else if !record[3].is_empty() {
                panic!("ETH addresses are not supported yet: {:?}", record);
            } else {
                panic!("No valid wallet address found in row: {:?}", record);
            };

            let schedule = VestingSchedule {
                cliff_amount: (total_amount as f64 * cliff_percentage) as u64,
                cliff_timestamp: cliff_timestamp.0,
                end_timestamp: end_timestamp.0,
                claimed_amount: 0,
                total_amount,
                recipient,
            };

            schedules.push(schedule);
        }
        // take the sum of all total_amounts
        let total_sum: u64 = schedules.iter().map(|s| s.total_amount).sum();
        println!("Total sum of all vesting amounts: {}", total_sum);
        // add one more schedule with the remaining amount
        let remaining_amount = TOTAL_AMOUNT_VESTED - total_sum;
        println!("Remaining amount: {}", remaining_amount);
        // treasury vesting schedule
        schedules.push(VestingSchedule {
            cliff_amount: remaining_amount,
            cliff_timestamp: now_and_5_minutes.0, // cliff timestamp is now + 5 minutes
            end_timestamp: end_timestamp.0,
            claimed_amount: 0,
            total_amount: remaining_amount,
            recipient: treasury_identity,
        });
        schedules
    }
}
