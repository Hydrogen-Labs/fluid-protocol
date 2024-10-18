pub mod utils {
    use fuels::accounts::{provider::Provider, wallet::WalletUnlocked};
    use fuels::prelude::*;
    use fuels::types::bech32::Bech32ContractId;
    use fuels::types::{Bits256, U256};
    use std::str::FromStr;
    use test_utils::data_structures::AssetContracts;

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
    pub async fn setup_wallet() -> WalletUnlocked {
        let rpc = match std::env::var("RPC") {
            Ok(s) => s,
            Err(error) => panic!("❌ Cannot find .env file: {:#?}", error),
        };
        println!("RPC: {}", rpc);

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

    pub fn load_core_contracts(wallet: WalletUnlocked) -> ProtocolContracts<WalletUnlocked> {
        let json = std::fs::read_to_string("contracts.json").unwrap();
        let contracts: serde_json::Value = serde_json::from_str(&json).unwrap();

        let borrow_operations_contract_id: Bech32ContractId = contracts["borrow_operations"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let borrow_operations =
            BorrowOperations::new(borrow_operations_contract_id, wallet.clone());

        let usdf_contract_id: Bech32ContractId =
            contracts["usdf"].as_str().unwrap().parse().unwrap();
        let usdf =
            test_utils::interfaces::usdf_token::USDFToken::new(usdf_contract_id, wallet.clone());

        let stability_pool_contract_id: Bech32ContractId = contracts["stability_pool"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let stability_pool = StabilityPool::new(stability_pool_contract_id, wallet.clone());

        let protocol_manager_contract_id: Bech32ContractId = contracts["protocol_manager"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let protocol_manager = ProtocolManager::new(protocol_manager_contract_id, wallet.clone());

        let fpt_staking_contract_id: Bech32ContractId =
            contracts["fpt_staking"].as_str().unwrap().parse().unwrap();
        let fpt_staking = FPTStaking::new(fpt_staking_contract_id, wallet.clone());

        let fpt_token_contract_id: Bech32ContractId =
            contracts["fpt_token"].as_str().unwrap().parse().unwrap();
        let fpt_token = FPTToken::new(fpt_token_contract_id.clone(), wallet.clone());

        let community_issuance_contract_id: Bech32ContractId = contracts["community_issuance"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let community_issuance =
            CommunityIssuance::new(community_issuance_contract_id, wallet.clone());

        let coll_surplus_pool_contract_id: Bech32ContractId = contracts["coll_surplus_pool"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let coll_surplus_pool = CollSurplusPool::new(coll_surplus_pool_contract_id, wallet.clone());

        let default_pool_contract_id: Bech32ContractId =
            contracts["default_pool"].as_str().unwrap().parse().unwrap();
        let default_pool = DefaultPool::new(default_pool_contract_id, wallet.clone());

        let active_pool_contract_id: Bech32ContractId =
            contracts["active_pool"].as_str().unwrap().parse().unwrap();
        let active_pool = ActivePool::new(active_pool_contract_id, wallet.clone());

        let sorted_troves_contract_id: Bech32ContractId = contracts["sorted_troves"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let sorted_troves = SortedTroves::new(sorted_troves_contract_id, wallet.clone());

        let vesting_contract_id: Bech32ContractId = contracts["vesting_contract"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let vesting_contract = VestingContract::new(vesting_contract_id, wallet.clone());

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
                let trove_manager_contract_id: Bech32ContractId = asset_contract["trove_manager"]
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
                    .unwrap()
                    .parse()
                    .unwrap();

                AssetContracts {
                    asset: Token::new(asset_contract_id, wallet.clone()),
                    asset_id,
                    oracle: test_utils::interfaces::oracle::Oracle::new(
                        oracle_contract_id,
                        wallet.clone(),
                    ),
                    trove_manager: TroveManagerContract::new(
                        trove_manager_contract_id,
                        wallet.clone(),
                    ),
                    mock_pyth_oracle: PythCore::new(pyth_contract_id, wallet.clone()),
                    mock_redstone_oracle: Some(RedstoneCore::new(
                        redstone_contract_id,
                        wallet.clone(),
                    )),
                    pyth_price_id: Bits256::from_hex_str(
                        asset_contract["pyth_price_id"].as_str().unwrap(),
                    )
                    .unwrap(),
                    pyth_precision: asset_contract["pyth_precision"].as_u64().unwrap() as u8,
                    redstone_precision: asset_contract["redstone_precision"].as_u64().unwrap()
                        as u8,
                    redstone_price_id: U256::from_str(
                        asset_contract["redstone_price_id"].as_str().unwrap(),
                    )
                    .unwrap(),
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
}
