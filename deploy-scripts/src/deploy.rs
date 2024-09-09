use std::{fs::File, io::Write, str::FromStr};

use dotenv::dotenv;
use fuels::prelude::*;
use serde_json::json;
use test_utils::interfaces::{
    active_pool::ActivePool, borrow_operations::BorrowOperations,
    coll_surplus_pool::CollSurplusPool, community_issuance::community_issuance_abi,
    default_pool::DefaultPool, fpt_staking::FPTStaking, fpt_token::fpt_token_abi, oracle::Oracle,
    protocol_manager::ProtocolManager, pyth_oracle::PythCore, redstone_oracle::RedstoneCore,
    sorted_troves::SortedTroves, stability_pool::StabilityPool, token::Token,
    trove_manager::TroveManagerContract, usdf_token::USDFToken,
};
use test_utils::setup::common::ExistingAssetContracts;

pub mod deployment {

    use fuels::types::Bits256;
    use fuels::{prelude::Account, types::Identity};
    use pbr::ProgressBar;
    use test_utils::interfaces::pyth_oracle::{pyth_oracle_abi, PythPrice, PythPriceFeed};

    use super::*;

    use test_utils::data_structures::PRECISION;
    use test_utils::interfaces::{
        active_pool::active_pool_abi, borrow_operations::borrow_operations_abi,
        coll_surplus_pool::coll_surplus_pool_abi, default_pool::default_pool_abi,
        fpt_staking::fpt_staking_abi, oracle::oracle_abi, protocol_manager::protocol_manager_abi,
        sorted_troves::sorted_troves_abi, stability_pool::stability_pool_abi, token::token_abi,
        trove_manager::trove_manager_abi, usdf_token::usdf_token_abi,
    };
    use test_utils::setup::common::{
        deploy_active_pool, deploy_borrow_operations, deploy_coll_surplus_pool,
        deploy_community_issuance, deploy_default_pool, deploy_fpt_staking, deploy_fpt_token,
        deploy_mock_pyth_oracle, deploy_mock_redstone_oracle, deploy_oracle,
        deploy_protocol_manager, deploy_sorted_troves, deploy_stability_pool, deploy_token,
        deploy_trove_manager_contract, deploy_usdf_token, deploy_vesting_contract, AssetContracts,
        ProtocolContracts,
    };

    pub async fn deploy() {
        dotenv().ok();
        let rpc = match std::env::var("RPC") {
            Ok(s) => s,
            Err(error) => panic!("❌ Cannot find .env file: {:#?}", error),
        };

        //--------------- WALLET ---------------
        let provider = match Provider::connect(rpc).await {
            Ok(p) => p,
            Err(error) => panic!("❌ Problem creating provider: {:#?}", error),
        };

        let secret = match std::env::var("SECRET") {
            Ok(s) => s,
            Err(error) => panic!("❌ Cannot find .env file: {:#?}", error),
        };

        let wallet = WalletUnlocked::new_from_mnemonic_phrase_with_path(
            &secret,
            Some(provider.clone()),
            "m/44'/1179993420'/0'/0/0",
        )
        .unwrap();

        let address = wallet.address();
        println!("🔑 Wallet address: {}", address);

        //--------------- Assets ---------------

        let eth_contracts = ExistingAssetContracts {
            asset: ContractId::from(
                Bech32ContractId::from_str(
                    "fuel1ql6d5vjmuqs0v2tev7su73zjrpajffy9cjccvll38mxmamaeteuqml4pxl",
                )
                .unwrap(),
            ),
            oracle: ContractId::from(
                Bech32ContractId::from_str(
                    "fuel129gw5u3rlacka3smhngevvgq4awllx8u4l5fktpr506yaxv8gx4qz6y4k3",
                )
                .unwrap(),
            ),
            pyth_oracle: ContractId::from(
                // TODO: change id?
                Bech32ContractId::from_str(
                    "fuel129gw5u3rlacka3smhngevvgq4awllx8u4l5fktpr506yaxv8gx4qz6y4k3",
                )
                .unwrap(),
            ),
            redstone_oracle: ContractId::from(
                // TODO: change id?
                Bech32ContractId::from_str(
                    "fuel129gw5u3rlacka3smhngevvgq4awllx8u4l5fktpr506yaxv8gx4qz6y4k3",
                )
                .unwrap(),
            ),
        };

        let st_eth_contracts = ExistingAssetContracts {
            asset: ContractId::from(
                Bech32ContractId::from_str(
                    "fuel1hud0p86m45k2qvhqpqwlz6c2h2pgj32w8tqhq0240dp6y2q26pvqg802xv",
                )
                .unwrap(),
            ),
            oracle: ContractId::from(
                Bech32ContractId::from_str(
                    "fuel1apa7t7dhpajrxg8xt4thmmaqq6378j4g8femnsz6u6etu3aeajksjzsdld",
                )
                .unwrap(),
            ),
            pyth_oracle: ContractId::from(
                // TODO: change id?
                Bech32ContractId::from_str(
                    "fuel129gw5u3rlacka3smhngevvgq4awllx8u4l5fktpr506yaxv8gx4qz6y4k3",
                )
                .unwrap(),
            ),
            redstone_oracle: ContractId::from(
                // TODO: change id?
                Bech32ContractId::from_str(
                    "fuel129gw5u3rlacka3smhngevvgq4awllx8u4l5fktpr506yaxv8gx4qz6y4k3",
                )
                .unwrap(),
            ),
        };

        //--------------- Deploy ---------------
        // TODO: Figure out max size
        // TODO: timestamp for pyth needs to be set somewhere
        let pyth_timestamp = 1;
        let contracts = deployment::deploy_and_initialize_all(
            wallet,
            100_000,
            true,
            Some(eth_contracts),
            Some(st_eth_contracts),
            pyth_timestamp,
        )
        .await;

        //--------------- Write to file ---------------
        let mut file = File::create("contracts.json").unwrap();

        let json = json!({
            "borrow_operations": contracts.borrow_operations.contract_id().to_string(),
            "usdf": contracts.usdf.contract_id().to_string(),
            "usdf_asset_id": contracts.usdf.contract_id().asset_id(&AssetId::zeroed().into()).to_string(),
            "stability_pool": contracts.stability_pool.contract_id().to_string(),
            "protocol_manager": contracts.protocol_manager.contract_id().to_string(),
            "fpt_staking": contracts.fpt_staking.contract_id().to_string(),
            "fpt_token": contracts.fpt_token.contract_id().to_string(),
            "fpt_asset_id": contracts.fpt_token.contract_id().asset_id(&AssetId::zeroed().into()).to_string(),
            "community_issuance": contracts.community_issuance.contract_id().to_string(),
            "coll_surplus_pool": contracts.coll_surplus_pool.contract_id().to_string(),
            "default_pool": contracts.default_pool.contract_id().to_string(),
            "active_pool": contracts.active_pool.contract_id().to_string(),
            "sorted_troves": contracts.sorted_troves.contract_id().to_string(),
            "vesting_contract": contracts.vesting_contract.contract_id().to_string(),
            "asset_contracts" : contracts.asset_contracts.iter().map(|asset_contracts| {
                json!({
                    "oracle": asset_contracts.oracle.contract_id().to_string(),
                    "trove_manager": asset_contracts.trove_manager.contract_id().to_string(),
                    "asset_contract": asset_contracts.asset.contract_id().to_string(),
                    "asset_id": asset_contracts.asset_id.to_string(),
                })
            }).collect::<Vec<serde_json::Value>>()
        });

        file.write_all(serde_json::to_string_pretty(&json).unwrap().as_bytes())
            .unwrap();
    }

    pub async fn deploy_and_initialize_all(
        wallet: WalletUnlocked,
        max_size: u64,
        deploy_2nd_asset: bool,
        existing_eth_contracts: Option<ExistingAssetContracts>,
        existing_st_eth_contracts: Option<ExistingAssetContracts>,
        pyth_timestamp: u64,
    ) -> ProtocolContracts<WalletUnlocked> {
        println!("Deploying parent contracts...");
        let mut pb = ProgressBar::new(13);

        let borrow_operations = deploy_borrow_operations(&wallet).await;
        pb.inc();

        let usdf = deploy_usdf_token(&wallet).await;
        pb.inc();

        let fpt_token = deploy_fpt_token(&wallet).await;
        pb.inc();

        let _fpt = deploy_token(&wallet).await;
        pb.inc();

        let fpt_staking = deploy_fpt_staking(&wallet).await;
        pb.inc();

        let stability_pool = deploy_stability_pool(&wallet).await;
        pb.inc();

        let protocol_manager = deploy_protocol_manager(&wallet).await;
        pb.inc();

        let community_issuance = deploy_community_issuance(&wallet).await;
        pb.inc();

        let coll_surplus_pool = deploy_coll_surplus_pool(&wallet).await;
        pb.inc();

        let default_pool = deploy_default_pool(&wallet).await;
        pb.inc();

        let active_pool = deploy_active_pool(&wallet).await;
        pb.inc();

        let sorted_troves = deploy_sorted_troves(&wallet).await;
        pb.inc();

        let vesting_contract = deploy_vesting_contract(&wallet).await;
        pb.inc();

        let pyth = deploy_mock_pyth_oracle(&wallet).await;
        let redstone = deploy_mock_pyth_oracle(&wallet).await;

        println!("Borrow operations: {}", borrow_operations.contract_id());
        println!("USDF Token: {}", usdf.contract_id());
        println!("Stability Pool: {}", stability_pool.contract_id());
        println!("FPT Staking: {}", fpt_staking.contract_id());
        println!("FPT Token: {}", fpt_token.contract_id());
        println!("Community Issuance {}", community_issuance.contract_id());
        println!("Coll Surplus Pool {}", coll_surplus_pool.contract_id());
        println!("Protocol Manager {}", protocol_manager.contract_id());
        println!("Default Pool {}", default_pool.contract_id());
        println!("Active Pool {}", active_pool.contract_id());
        println!("Sorted Troves {}", sorted_troves.contract_id());
        println!("Initializing contracts...");

        let fuel_asset_contracts = upload_asset(wallet.clone(), &existing_eth_contracts).await;

        let mut pb = ProgressBar::new(7);

        let mut asset_contracts: Vec<AssetContracts<WalletUnlocked>> = vec![];
        wait();

        let _ = community_issuance_abi::initialize(
            &community_issuance,
            stability_pool.contract_id().into(),
            fpt_token
                .contract_id()
                .asset_id(&AssetId::zeroed().into())
                .into(),
            &Identity::Address(wallet.address().into()),
            false,
        )
        .await;
        pb.inc();

        fpt_token_abi::initialize(&fpt_token, &vesting_contract, &community_issuance, false).await;
        pb.inc();

        let _ = usdf_token_abi::initialize(
            &usdf,
            protocol_manager.contract_id().into(),
            Identity::ContractId(stability_pool.contract_id().into()),
            Identity::ContractId(borrow_operations.contract_id().into()),
        )
        .await;
        pb.inc();

        let _ = borrow_operations_abi::initialize(
            &borrow_operations,
            usdf.contract_id().into(),
            fpt_staking.contract_id().into(),
            protocol_manager.contract_id().into(),
            coll_surplus_pool.contract_id().into(),
            active_pool.contract_id().into(),
            sorted_troves.contract_id().into(),
        )
        .await;
        wait();
        pb.inc();

        let _ = stability_pool_abi::initialize(
            &stability_pool,
            usdf.contract_id().into(),
            community_issuance.contract_id().into(),
            protocol_manager.contract_id().into(),
            active_pool.contract_id().into(),
        )
        .await
        .unwrap();
        wait();
        pb.inc();

        let _ = fpt_staking_abi::initialize(
            &fpt_staking,
            protocol_manager.contract_id().into(),
            borrow_operations.contract_id().into(),
            fpt_token
                .contract_id()
                .asset_id(&AssetId::zeroed().into())
                .into(),
            usdf.contract_id()
                .asset_id(&AssetId::zeroed().into())
                .into(),
        )
        .await;
        wait();
        pb.inc();

        let _ = coll_surplus_pool_abi::initialize(
            &coll_surplus_pool,
            borrow_operations.contract_id().into(),
            Identity::ContractId(protocol_manager.contract_id().into()),
        )
        .await;
        wait();
        pb.inc();

        let _ = protocol_manager_abi::initialize(
            &protocol_manager,
            borrow_operations.contract_id().into(),
            stability_pool.contract_id().into(),
            fpt_staking.contract_id().into(),
            usdf.contract_id().into(),
            coll_surplus_pool.contract_id().into(),
            default_pool.contract_id().into(),
            active_pool.contract_id().into(),
            sorted_troves.contract_id().into(),
            Identity::Address(wallet.address().into()),
        )
        .await;
        wait();
        pb.inc();

        let _ = default_pool_abi::initialize(
            &default_pool,
            Identity::ContractId(protocol_manager.contract_id().into()),
            active_pool.contract_id().into(),
        )
        .await;
        wait();
        pb.inc();

        let _ = active_pool_abi::initialize(
            &active_pool,
            Identity::ContractId(borrow_operations.contract_id().into()),
            Identity::ContractId(stability_pool.contract_id().into()),
            default_pool.contract_id().into(),
            Identity::ContractId(protocol_manager.contract_id().into()),
        )
        .await;
        wait();
        pb.inc();

        // TODO: Verify max size is correct
        let _ = sorted_troves_abi::initialize(
            &sorted_troves,
            max_size,
            protocol_manager.contract_id().into(),
            borrow_operations.contract_id().into(),
        )
        .await;
        wait();
        pb.inc();

        initialize_asset(
            &borrow_operations,
            &fpt_staking,
            &stability_pool,
            &protocol_manager,
            &usdf,
            &coll_surplus_pool,
            wallet.clone(),
            "Fuel".to_string(),
            "FUEL".to_string(),
            &default_pool,
            &active_pool,
            &fuel_asset_contracts.asset,
            &fuel_asset_contracts.trove_manager,
            &sorted_troves,
            &fuel_asset_contracts.oracle,
            &pyth,
            pyth_timestamp,
            existing_eth_contracts,
        )
        .await;

        if deploy_2nd_asset {
            let stfuel_asset_contracts =
                upload_asset(wallet.clone(), &existing_st_eth_contracts).await;

            initialize_asset(
                &borrow_operations,
                &fpt_staking,
                &stability_pool,
                &protocol_manager,
                &usdf,
                &coll_surplus_pool,
                wallet.clone(),
                "stFuel".to_string(),
                "stFUEL".to_string(),
                &default_pool,
                &active_pool,
                &stfuel_asset_contracts.asset,
                &stfuel_asset_contracts.trove_manager,
                &sorted_troves,
                &stfuel_asset_contracts.oracle,
                &pyth,
                pyth_timestamp,
                existing_st_eth_contracts,
            )
            .await;

            asset_contracts.push(stfuel_asset_contracts);
        }
        pb.finish();

        asset_contracts.push(fuel_asset_contracts);

        let contracts = ProtocolContracts {
            borrow_operations,
            usdf,
            stability_pool,
            protocol_manager,
            asset_contracts,
            fpt_staking,
            fpt_token,
            community_issuance,
            coll_surplus_pool,
            default_pool,
            sorted_troves,
            active_pool,
            vesting_contract,
        };

        return contracts;
    }

    pub fn wait() {
        // Necessary for random instances where the 'UTXO' cannot be found
        std::thread::sleep(std::time::Duration::from_secs(15));
    }

    pub async fn upload_asset(
        wallet: WalletUnlocked,
        existing_contracts: &Option<ExistingAssetContracts>,
    ) -> AssetContracts<WalletUnlocked> {
        println!("Deploying asset contracts...");
        let mut pb = ProgressBar::new(3);
        let trove_manager = deploy_trove_manager_contract(&wallet).await;
        pb.inc();

        match existing_contracts {
            Some(contracts) => {
                pb.finish();
                let asset = Token::new(contracts.asset, wallet.clone());
                let asset_id: AssetId = asset
                    .contract_id()
                    .asset_id(&AssetId::zeroed().into())
                    .into();

                return AssetContracts {
                    oracle: Oracle::new(contracts.oracle, wallet.clone()),
                    mock_pyth_oracle: PythCore::new(contracts.pyth_oracle, wallet.clone()),
                    mock_redstone_oracle: RedstoneCore::new(
                        contracts.redstone_oracle,
                        wallet.clone(),
                    ),
                    asset,
                    trove_manager,
                    asset_id,
                };
            }
            None => {
                let pyth = deploy_mock_pyth_oracle(&wallet).await;
                let redstone = deploy_mock_redstone_oracle(&wallet).await;
                let oracle = deploy_oracle(
                    &wallet,
                    pyth.contract_id().into(),
                    9,
                    redstone.contract_id().into(),
                    9,
                )
                .await;
                pb.inc();
                let asset = deploy_token(&wallet).await;
                pb.inc();

                let asset_id: AssetId = asset
                    .contract_id()
                    .asset_id(&AssetId::zeroed().into())
                    .into();

                println!("Deploying asset contracts... Done");
                println!("Oracle: {}", oracle.contract_id());
                println!("Mock Pyth Oracle: {}", pyth.contract_id());
                println!("Mock Redstone Oracle: {}", redstone.contract_id());
                println!("Trove Manager: {}", trove_manager.contract_id());
                println!("Asset: {}", asset.contract_id());

                return AssetContracts {
                    oracle,
                    mock_pyth_oracle: pyth,
                    mock_redstone_oracle: redstone,
                    trove_manager,
                    asset,
                    asset_id,
                };
            }
        }
    }

    pub async fn initialize_asset<T: Account>(
        borrow_operations: &BorrowOperations<T>,
        fpt_staking: &FPTStaking<T>,
        stability_pool: &StabilityPool<T>,
        protocol_manager: &ProtocolManager<T>,
        usdf: &USDFToken<T>,
        coll_surplus_pool: &CollSurplusPool<T>,
        wallet: WalletUnlocked,
        name: String,
        symbol: String,
        default_pool: &DefaultPool<T>,
        active_pool: &ActivePool<T>,
        asset: &Token<T>,
        trove_manager: &TroveManagerContract<T>,
        sorted_troves: &SortedTroves<T>,
        oracle: &Oracle<T>,
        pyth: &PythCore<T>,
        pyth_publish_time: u64,
        existing_contracts: Option<ExistingAssetContracts>,
    ) -> () {
        println!("Initializing asset contracts...");
        let mut pb = ProgressBar::new(7);

        match existing_contracts {
            Some(_) => {}
            None => {
                let _ = token_abi::initialize(
                    &asset,
                    1_000_000_000,
                    &Identity::Address(wallet.address().into()),
                    name.to_string(),
                    symbol.to_string(),
                )
                .await;
                wait();
                pb.inc();

                let pyth_feed = vec![(
                    Bits256::zeroed(),
                    PythPriceFeed {
                        price: PythPrice {
                            price: 1_000 * PRECISION,
                            publish_time: pyth_publish_time,
                        },
                    },
                )];

                pyth_oracle_abi::update_price_feeds(&pyth, pyth_feed).await;
                wait();
                pb.inc();
            }
        }

        let _ = trove_manager_abi::initialize(
            &trove_manager,
            borrow_operations.contract_id().into(),
            sorted_troves.contract_id().into(),
            oracle.contract_id().into(),
            stability_pool.contract_id().into(),
            default_pool.contract_id().into(),
            active_pool.contract_id().into(),
            coll_surplus_pool.contract_id().into(),
            usdf.contract_id().into(),
            asset
                .contract_id()
                .asset_id(&AssetId::zeroed().into())
                .into(),
            protocol_manager.contract_id().into(),
        )
        .await;
        wait();
        pb.inc();

        let _ = protocol_manager_abi::register_asset(
            &protocol_manager,
            asset
                .contract_id()
                .asset_id(&AssetId::zeroed().into())
                .into(),
            trove_manager.contract_id().into(),
            oracle.contract_id().into(),
            borrow_operations,
            stability_pool,
            usdf,
            fpt_staking,
            coll_surplus_pool,
            default_pool,
            active_pool,
            sorted_troves,
        )
        .await;
        wait();
        pb.inc();
    }
}
