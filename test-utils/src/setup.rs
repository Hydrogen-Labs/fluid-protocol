use super::interfaces::{
    active_pool::ActivePool,
    borrow_operations::BorrowOperations,
    coll_surplus_pool::CollSurplusPool,
    community_issuance::CommunityIssuance,
    default_pool::DefaultPool,
    fpt_staking::FPTStaking,
    fpt_token::FPTToken,
    hint_helper::HintHelper,
    oracle::{Oracle, OracleConfigurables},
    protocol_manager::ProtocolManager,
    pyth_oracle::{PythCore, PythPrice, PythPriceFeed, DEFAULT_PYTH_PRICE_ID, PYTH_TIMESTAMP},
    redstone_oracle::{RedstoneCore, DEFAULT_REDSTONE_PRICE_ID},
    sorted_troves::SortedTroves,
    stability_pool::StabilityPool,
    token::Token,
    trove_manager::TroveManagerContract,
    usdf_token::USDFToken,
    vesting::VestingContract,
};

use fuels::prelude::{Contract, TxPolicies, WalletUnlocked};

pub mod common {
    use super::*;
    use crate::{
        data_structures::{AssetContracts, ExistingAssetContracts, ProtocolContracts, PRECISION},
        interfaces::{
            active_pool::active_pool_abi,
            borrow_operations::borrow_operations_abi,
            coll_surplus_pool::coll_surplus_pool_abi,
            community_issuance::community_issuance_abi,
            default_pool::default_pool_abi,
            fpt_staking::fpt_staking_abi,
            fpt_token::fpt_token_abi,
            oracle::oracle_abi,
            protocol_manager::protocol_manager_abi,
            pyth_oracle::{pyth_oracle_abi, pyth_price_feed},
            redstone_oracle::{redstone_oracle_abi, redstone_price_feed_with_id},
            sorted_troves::sorted_troves_abi,
            stability_pool::stability_pool_abi,
            token::token_abi,
            trove_manager::trove_manager_abi,
            usdf_token::usdf_token_abi,
        },
        paths::*,
    };
    use fuels::{
        // accounts::rand::{self, Rng},
        prelude::*,
        programs::responses::CallResponse,
        types::{Bits256, ContractId, Identity, U256},
    };
    use pbr::ProgressBar;
    // use pbr::ProgressBar;
    use rand::Rng;
    use std::env;

    pub async fn setup_protocol(
        num_wallets: u64,
        deploy_2nd_asset: bool,
        use_test_fpt: bool,
    ) -> (
        ProtocolContracts<WalletUnlocked>,
        WalletUnlocked,
        Vec<WalletUnlocked>,
    ) {
        // Launch a local network and deploy the contract
        let mut wallets = launch_custom_provider_and_get_wallets(
            WalletsConfig::new(
                Some(num_wallets),   /* Single wallet */
                Some(1),             /* Single coin (UTXO) */
                Some(1_000_000_000), /* Amount per coin */
            ),
            None,
            None,
        )
        .await
        .unwrap();
        let wallet = wallets.pop().unwrap();

        let mut contracts = deploy_core_contracts(&wallet, use_test_fpt).await;
        initialize_core_contracts(&mut contracts, &wallet, use_test_fpt, true).await;

        // Add the first asset (Fuel)
        let mock_asset_contracts = add_asset(
            &mut contracts,
            &wallet,
            "MOCK".to_string(),
            "MCK".to_string(),
        )
        .await;
        contracts.asset_contracts.push(mock_asset_contracts);

        // Add the second asset if required, we often don't deploy the second asset to save on test time
        if deploy_2nd_asset {
            let rock_asset_contracts = add_asset(
                &mut contracts,
                &wallet,
                "ROCK".to_string(),
                "RCK".to_string(),
            )
            .await;
            contracts.asset_contracts.push(rock_asset_contracts);
        }

        (contracts, wallet, wallets)
    }

    pub async fn deploy_core_contracts(
        wallet: &WalletUnlocked,
        use_test_fpt: bool,
    ) -> ProtocolContracts<WalletUnlocked> {
        println!("Deploying core contracts...");

        let borrow_operations = deploy_borrow_operations(wallet).await;
        let usdf = deploy_usdf_token(wallet).await;
        let stability_pool = deploy_stability_pool(wallet).await;
        let fpt_staking = deploy_fpt_staking(wallet).await;
        let community_issuance = deploy_community_issuance(wallet).await;
        let fpt_token = if use_test_fpt {
            deploy_test_fpt_token(wallet).await
        } else {
            deploy_fpt_token(wallet).await
        };
        let protocol_manager = deploy_protocol_manager(wallet).await;
        let coll_surplus_pool = deploy_coll_surplus_pool(wallet).await;
        let default_pool = deploy_default_pool(wallet).await;
        let active_pool = deploy_active_pool(wallet).await;
        let sorted_troves = deploy_sorted_troves(wallet).await;
        let vesting_contract = deploy_vesting_contract(wallet).await;

        let fpt_asset_id = fpt_token.contract_id().asset_id(&AssetId::zeroed().into());
        let usdf_asset_id = usdf.contract_id().asset_id(&AssetId::zeroed().into());

        ProtocolContracts {
            borrow_operations,
            usdf,
            stability_pool,
            asset_contracts: vec![],
            protocol_manager,
            fpt_staking,
            fpt_token,
            fpt_asset_id,
            usdf_asset_id,
            coll_surplus_pool,
            default_pool,
            active_pool,
            sorted_troves,
            community_issuance,
            vesting_contract,
        }
    }

    pub async fn initialize_core_contracts(
        contracts: &mut ProtocolContracts<WalletUnlocked>,
        wallet: &WalletUnlocked,
        use_test_fpt: bool,
        debug: bool,
    ) {
        println!("Initializing core contracts...");
        if !use_test_fpt {
            fpt_token_abi::initialize(
                &contracts.fpt_token,
                &contracts.vesting_contract,
                &contracts.community_issuance,
            )
            .await;
        }

        community_issuance_abi::initialize(
            &contracts.community_issuance,
            contracts.stability_pool.contract_id().into(),
            contracts.fpt_asset_id,
            &Identity::Address(wallet.address().into()),
            debug,
        )
        .await
        .unwrap();

        usdf_token_abi::initialize(
            &contracts.usdf,
            contracts.protocol_manager.contract_id().into(),
            Identity::ContractId(contracts.stability_pool.contract_id().into()),
            Identity::ContractId(contracts.borrow_operations.contract_id().into()),
        )
        .await
        .unwrap();

        borrow_operations_abi::initialize(
            &contracts.borrow_operations,
            contracts.usdf.contract_id().into(),
            contracts.fpt_staking.contract_id().into(),
            contracts.protocol_manager.contract_id().into(),
            contracts.coll_surplus_pool.contract_id().into(),
            contracts.active_pool.contract_id().into(),
            contracts.sorted_troves.contract_id().into(),
        )
        .await;

        stability_pool_abi::initialize(
            &contracts.stability_pool,
            contracts.usdf.contract_id().into(),
            contracts.community_issuance.contract_id().into(),
            contracts.protocol_manager.contract_id().into(),
            contracts.active_pool.contract_id().into(),
            contracts.sorted_troves.contract_id().into(),
        )
        .await
        .unwrap();

        fpt_staking_abi::initialize(
            &contracts.fpt_staking,
            contracts.protocol_manager.contract_id().into(),
            contracts.borrow_operations.contract_id().into(),
            contracts.fpt_asset_id,
            contracts
                .usdf
                .contract_id()
                .asset_id(&AssetId::zeroed().into())
                .into(),
        )
        .await;

        protocol_manager_abi::initialize(
            &contracts.protocol_manager,
            contracts.borrow_operations.contract_id().into(),
            contracts.stability_pool.contract_id().into(),
            contracts.fpt_staking.contract_id().into(),
            contracts.usdf.contract_id().into(),
            contracts.coll_surplus_pool.contract_id().into(),
            contracts.default_pool.contract_id().into(),
            contracts.active_pool.contract_id().into(),
            contracts.sorted_troves.contract_id().into(),
            Identity::Address(wallet.address().into()),
        )
        .await;

        coll_surplus_pool_abi::initialize(
            &contracts.coll_surplus_pool,
            contracts.borrow_operations.contract_id().into(),
            Identity::ContractId(contracts.protocol_manager.contract_id().into()),
        )
        .await
        .unwrap();

        default_pool_abi::initialize(
            &contracts.default_pool,
            Identity::ContractId(contracts.protocol_manager.contract_id().into()),
            contracts.active_pool.contract_id().into(),
        )
        .await
        .unwrap();

        active_pool_abi::initialize(
            &contracts.active_pool,
            Identity::ContractId(contracts.borrow_operations.contract_id().into()),
            Identity::ContractId(contracts.stability_pool.contract_id().into()),
            contracts.default_pool.contract_id().into(),
            Identity::ContractId(contracts.protocol_manager.contract_id().into()),
        )
        .await
        .unwrap();

        sorted_troves_abi::initialize(
            &contracts.sorted_troves,
            100_000_000,
            contracts.protocol_manager.contract_id().into(),
            contracts.borrow_operations.contract_id().into(),
        )
        .await
        .unwrap();
    }

    async fn deploy_test_fpt_token(wallet: &WalletUnlocked) -> FPTToken<WalletUnlocked> {
        let mock_fpt_token = deploy_token(wallet).await;

        token_abi::initialize(
            &mock_fpt_token,
            1_000_000_000,
            &Identity::Address(wallet.address().into()),
            "Mock FPT Token".to_string(),
            "mFPT".to_string(),
        )
        .await
        .unwrap();

        FPTToken::new(mock_fpt_token.contract_id().clone(), wallet.clone())
    }

    pub async fn deploy_token(wallet: &WalletUnlocked) -> Token<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(TOKEN_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await;

        match id {
            Ok(id) => return Token::new(id.clone(), wallet.clone()),
            Err(_) => {
                wait();
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(TOKEN_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default().with_salt(salt),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policies)
                .await
                .unwrap();

                return Token::new(id.clone(), wallet.clone());
            }
        }
    }

    pub async fn deploy_fpt_token(wallet: &WalletUnlocked) -> FPTToken<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(FPT_TOKEN_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await;

        match id {
            Ok(id) => return FPTToken::new(id, wallet.clone()),
            Err(_) => {
                wait();
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(FPT_TOKEN_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default().with_salt(salt),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policies)
                .await
                .unwrap();

                return FPTToken::new(id, wallet.clone());
            }
        }
    }

    pub async fn deploy_sorted_troves(wallet: &WalletUnlocked) -> SortedTroves<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policy = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(SORTED_TROVES_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policy)
        .await;

        match id {
            Ok(id) => return SortedTroves::new(id, wallet.clone()),
            Err(_) => {
                wait();
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(SORTED_TROVES_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default().with_salt(salt),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policy)
                .await
                .unwrap();

                return SortedTroves::new(id, wallet.clone());
            }
        }
    }

    pub async fn deploy_trove_manager_contract(
        wallet: &WalletUnlocked,
    ) -> TroveManagerContract<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(TROVE_MANAGER_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await;

        match id {
            Ok(id) => return TroveManagerContract::new(id, wallet.clone()),
            Err(_) => {
                wait();
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(TROVE_MANAGER_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default().with_salt(salt),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policies)
                .await
                .unwrap();

                return TroveManagerContract::new(id, wallet.clone());
            }
        }
    }

    pub async fn deploy_vesting_contract(
        wallet: &WalletUnlocked,
    ) -> VestingContract<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(VESTING_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await;

        match id {
            Ok(id) => return VestingContract::new(id, wallet.clone()),
            Err(_) => {
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(VESTING_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default().with_salt(salt),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policies)
                .await
                .unwrap();

                return VestingContract::new(id, wallet.clone());
            }
        }
    }

    pub async fn deploy_mock_pyth_oracle(wallet: &WalletUnlocked) -> PythCore<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(PYTH_ORACLE_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await;

        match id {
            Ok(id) => {
                return PythCore::new(id, wallet.clone());
            }
            Err(_) => {
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(PYTH_ORACLE_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default().with_salt(salt),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policies)
                .await
                .unwrap();

                return PythCore::new(id, wallet.clone());
            }
        }
    }

    pub async fn deploy_mock_redstone_oracle(
        wallet: &WalletUnlocked,
    ) -> RedstoneCore<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(REDSTONE_ORACLE_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await;

        match id {
            Ok(id) => {
                return RedstoneCore::new(id, wallet.clone());
            }
            Err(_) => {
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(REDSTONE_ORACLE_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default().with_salt(salt),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policies)
                .await
                .unwrap();

                return RedstoneCore::new(id, wallet.clone());
            }
        }
    }

    pub async fn deploy_oracle(
        wallet: &WalletUnlocked,
        pyth: ContractId,
        pyth_precision: u8,
        pyth_price_id: Bits256,
        redstone: Option<ContractId>,
        redstone_precison: u32,
        redstone_price_id: U256,
        debug: bool,
    ) -> Oracle<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let configurables = OracleConfigurables::default()
            .with_PYTH(pyth)
            .unwrap()
            .with_PYTH_PRICE_ID(pyth_price_id)
            .unwrap()
            .with_REDSTONE(redstone)
            .unwrap()
            .with_REDSTONE_PRICE_ID(redstone_price_id)
            .unwrap()
            .with_DEBUG(debug)
            .unwrap()
            .with_PYTH_PRECISION(pyth_precision)
            .unwrap()
            .with_REDSTONE_PRECISION(redstone_precison)
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(ORACLE_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_salt(salt)
                .with_configurables(configurables.clone()),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await;

        match id {
            Ok(id) => {
                return Oracle::new(id, wallet.clone());
            }
            Err(_) => {
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(ORACLE_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default()
                        .with_salt(salt)
                        .with_configurables(configurables),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policies)
                .await
                .unwrap();

                return Oracle::new(id, wallet.clone());
            }
        }
    }

    pub async fn deploy_protocol_manager(
        wallet: &WalletUnlocked,
    ) -> ProtocolManager<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(PROTCOL_MANAGER_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await
        .unwrap();

        ProtocolManager::new(id, wallet.clone())
    }

    pub async fn deploy_borrow_operations(
        wallet: &WalletUnlocked,
    ) -> BorrowOperations<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(BORROW_OPERATIONS_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await;

        match id {
            Ok(id) => {
                return BorrowOperations::new(id, wallet.clone());
            }
            Err(_) => {
                wait();
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(BORROW_OPERATIONS_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default().with_salt(salt),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policies)
                .await
                .unwrap();

                return BorrowOperations::new(id, wallet.clone());
            }
        }
    }

    pub fn get_absolute_path_from_relative(relative_path: &str) -> String {
        let current_dir = env::current_dir().unwrap();

        let fluid_protocol_path = current_dir
            .ancestors()
            .find(|p| p.ends_with("fluid-protocol"))
            .unwrap_or(&current_dir)
            .to_path_buf();

        fluid_protocol_path
            .join(relative_path)
            .to_str()
            .unwrap()
            .to_string()
    }

    pub async fn deploy_asset_contracts(
        wallet: &WalletUnlocked,
        existing_contracts: &Option<ExistingAssetContracts>,
    ) -> AssetContracts<WalletUnlocked> {
        println!("Deploying asset contracts...");
        let mut pb = ProgressBar::new(6);
        let trove_manager = deploy_trove_manager_contract(&wallet).await;

        pb.inc();

        match existing_contracts {
            Some(contracts) => {
                pb.finish();

                let oracle = deploy_oracle(
                    &wallet,
                    contracts.pyth_oracle,
                    contracts.pyth_precision,
                    contracts.pyth_price_id,
                    contracts.redstone_oracle,
                    contracts.redstone_precision,
                    contracts.redstone_price_id,
                    false,
                )
                .await;

                return AssetContracts {
                    oracle,
                    mock_pyth_oracle: PythCore::new(contracts.pyth_oracle, wallet.clone()),
                    mock_redstone_oracle: match contracts.redstone_oracle {
                        Some(redstone) => Some(RedstoneCore::new(redstone, wallet.clone())),
                        None => None,
                    },
                    trove_manager,
                    asset: Token::new(contracts.asset, wallet.clone()),
                    asset_id: contracts.asset_id,
                    pyth_price_id: contracts.pyth_price_id,
                    pyth_precision: contracts.pyth_precision,
                    redstone_price_id: contracts.redstone_price_id,
                    redstone_precision: contracts.redstone_precision,
                };
            }
            None => {
                let asset = deploy_token(&wallet).await;
                pb.inc();
                let asset_id: AssetId = asset
                    .contract_id()
                    .asset_id(&AssetId::zeroed().into())
                    .into();

                token_abi::mint_to_id(
                    &asset,
                    5000 * PRECISION,
                    Identity::Address(wallet.address().into()),
                )
                .await;

                let pyth_price_id = Bits256::from(asset_id);
                let redstone_price_id = U256::from(rand::thread_rng().gen_range(1..1_000_000));

                let pyth = deploy_mock_pyth_oracle(&wallet).await;
                let redstone = deploy_mock_redstone_oracle(&wallet).await;
                let oracle = deploy_oracle(
                    &wallet,
                    pyth.contract_id().into(),
                    9,
                    pyth_price_id,
                    Some(redstone.contract_id().into()),
                    9,
                    redstone_price_id,
                    true,
                )
                .await;
                pb.inc();

                println!("Deploying asset contracts... Done");
                println!("Oracle: {}", oracle.contract_id());
                println!("Mock Pyth Oracle: {}", pyth.contract_id());
                println!("Mock Redstone Oracle: {}", redstone.contract_id());
                println!("Trove Manager: {}", trove_manager.contract_id());
                println!("Asset: {}", asset.contract_id());

                let _ = token_abi::initialize(
                    &asset,
                    1_000_000_000,
                    &Identity::Address(wallet.address().into()),
                    "MOCK".to_string(),
                    "MOCK".to_string(),
                )
                .await;
                pb.inc();
                let pyth_feed = vec![(
                    pyth_price_id,
                    PythPriceFeed {
                        price: PythPrice {
                            price: 1 * PRECISION,
                            publish_time: PYTH_TIMESTAMP,
                        },
                    },
                )];
                let redstone_feed = redstone_price_feed_with_id(redstone_price_id, vec![1]);

                oracle_abi::set_debug_timestamp(&oracle, PYTH_TIMESTAMP).await;
                pyth_oracle_abi::update_price_feeds(&pyth, pyth_feed).await;
                pb.inc();

                redstone_oracle_abi::write_prices(&redstone, redstone_feed).await;
                redstone_oracle_abi::set_timestamp(&redstone, PYTH_TIMESTAMP).await;
                pb.inc();

                return AssetContracts {
                    oracle,
                    mock_pyth_oracle: pyth,
                    mock_redstone_oracle: Some(redstone),
                    trove_manager,
                    asset,
                    asset_id,
                    pyth_price_id,
                    pyth_precision: 9,
                    redstone_price_id,
                    redstone_precision: 9,
                };
            }
        }
    }

    pub async fn add_asset(
        contracts: &mut ProtocolContracts<WalletUnlocked>,
        wallet: &WalletUnlocked,
        name: String,
        symbol: String,
    ) -> AssetContracts<WalletUnlocked> {
        let pyth = deploy_mock_pyth_oracle(wallet).await;
        let redstone = deploy_mock_redstone_oracle(wallet).await;
        let oracle = deploy_oracle(
            wallet,
            pyth.contract_id().into(),
            9,
            DEFAULT_PYTH_PRICE_ID,
            Some(redstone.contract_id().into()),
            9,
            DEFAULT_REDSTONE_PRICE_ID,
            true,
        )
        .await;
        let trove_manager = deploy_trove_manager_contract(wallet).await;
        let asset = deploy_token(wallet).await;

        token_abi::initialize(
            &asset,
            1_000_000_000,
            &Identity::Address(wallet.address().into()),
            name,
            symbol,
        )
        .await
        .unwrap();

        trove_manager_abi::initialize(
            &trove_manager,
            contracts.borrow_operations.contract_id().into(),
            contracts.sorted_troves.contract_id().into(),
            oracle.contract_id().into(),
            contracts.stability_pool.contract_id().into(),
            contracts.default_pool.contract_id().into(),
            contracts.active_pool.contract_id().into(),
            contracts.coll_surplus_pool.contract_id().into(),
            contracts.usdf.contract_id().into(),
            asset
                .contract_id()
                .asset_id(&AssetId::zeroed().into())
                .into(),
            contracts.protocol_manager.contract_id().into(),
        )
        .await
        .unwrap();

        pyth_oracle_abi::update_price_feeds(&pyth, pyth_price_feed(1)).await;

        protocol_manager_abi::register_asset(
            &contracts.protocol_manager,
            asset
                .contract_id()
                .asset_id(&AssetId::zeroed().into())
                .into(),
            trove_manager.contract_id().into(),
            oracle.contract_id().into(),
            &contracts.borrow_operations,
            &contracts.stability_pool,
            &contracts.usdf,
            &contracts.fpt_staking,
            &contracts.coll_surplus_pool,
            &contracts.default_pool,
            &contracts.active_pool,
            &contracts.sorted_troves,
        )
        .await;

        let asset_id: AssetId = asset
            .contract_id()
            .asset_id(&AssetId::zeroed().into())
            .into();

        AssetContracts {
            oracle,
            mock_pyth_oracle: pyth,
            mock_redstone_oracle: Some(redstone),
            trove_manager,
            asset,
            asset_id,
            pyth_price_id: DEFAULT_PYTH_PRICE_ID,
            pyth_precision: 9,
            redstone_price_id: DEFAULT_REDSTONE_PRICE_ID,
            redstone_precision: 9,
        }
    }

    pub async fn initialize_asset<T: Account>(
        core_protocol_contracts: &ProtocolContracts<T>,
        asset_contracts: &AssetContracts<T>,
    ) -> () {
        println!("Initializing asset contracts...");
        let mut pb = ProgressBar::new(2);

        let _ = trove_manager_abi::initialize(
            &asset_contracts.trove_manager,
            core_protocol_contracts
                .borrow_operations
                .contract_id()
                .into(),
            core_protocol_contracts.sorted_troves.contract_id().into(),
            asset_contracts.oracle.contract_id().into(),
            core_protocol_contracts.stability_pool.contract_id().into(),
            core_protocol_contracts.default_pool.contract_id().into(),
            core_protocol_contracts.active_pool.contract_id().into(),
            core_protocol_contracts
                .coll_surplus_pool
                .contract_id()
                .into(),
            core_protocol_contracts.usdf.contract_id().into(),
            asset_contracts.asset_id,
            core_protocol_contracts
                .protocol_manager
                .contract_id()
                .into(),
        )
        .await;
        pb.inc();

        let _ = protocol_manager_abi::register_asset(
            &core_protocol_contracts.protocol_manager,
            asset_contracts.asset_id,
            asset_contracts.trove_manager.contract_id().into(),
            asset_contracts.oracle.contract_id().into(),
            &core_protocol_contracts.borrow_operations,
            &core_protocol_contracts.stability_pool,
            &core_protocol_contracts.usdf,
            &core_protocol_contracts.fpt_staking,
            &core_protocol_contracts.coll_surplus_pool,
            &core_protocol_contracts.default_pool,
            &core_protocol_contracts.active_pool,
            &core_protocol_contracts.sorted_troves,
        )
        .await;
        pb.inc();
    }

    pub async fn deploy_active_pool(wallet: &WalletUnlocked) -> ActivePool<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(ACTIVE_POOL_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await;

        match id {
            Ok(id) => {
                return ActivePool::new(id, wallet.clone());
            }
            Err(_) => {
                wait();
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(ACTIVE_POOL_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default().with_salt(salt),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policies)
                .await
                .unwrap();

                return ActivePool::new(id, wallet.clone());
            }
        }
    }

    pub async fn deploy_stability_pool(wallet: &WalletUnlocked) -> StabilityPool<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(STABILITY_POOL_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await;

        match id {
            Ok(id) => {
                return StabilityPool::new(id, wallet.clone());
            }
            Err(_) => {
                wait();
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(STABILITY_POOL_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default().with_salt(salt),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policies)
                .await
                .unwrap();

                return StabilityPool::new(id, wallet.clone());
            }
        }
    }

    pub async fn deploy_default_pool(wallet: &WalletUnlocked) -> DefaultPool<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(DEFAULT_POOL_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await;

        match id {
            Ok(id) => {
                return DefaultPool::new(id, wallet.clone());
            }
            Err(_) => {
                wait();
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(DEFAULT_POOL_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default().with_salt(salt),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policies)
                .await
                .unwrap();

                return DefaultPool::new(id, wallet.clone());
            }
        }
    }

    pub async fn deploy_coll_surplus_pool(
        wallet: &WalletUnlocked,
    ) -> CollSurplusPool<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(COLL_SURPLUS_POOL_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await;

        match id {
            Ok(id) => {
                return CollSurplusPool::new(id, wallet.clone());
            }
            Err(_) => {
                wait();
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(COLL_SURPLUS_POOL_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default().with_salt(salt),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policies)
                .await
                .unwrap();

                return CollSurplusPool::new(id, wallet.clone());
            }
        }
    }

    pub async fn deploy_community_issuance(
        wallet: &WalletUnlocked,
    ) -> CommunityIssuance<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(COMMUNITY_ISSUANCE_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await;

        match id {
            Ok(id) => {
                return CommunityIssuance::new(id, wallet.clone());
            }
            Err(_) => {
                wait();
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(COMMUNITY_ISSUANCE_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default().with_salt(salt),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policies)
                .await
                .unwrap();

                return CommunityIssuance::new(id, wallet.clone());
            }
        }
    }

    pub async fn deploy_fpt_staking(wallet: &WalletUnlocked) -> FPTStaking<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(FPT_STAKING_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await;

        match id {
            Ok(id) => {
                return FPTStaking::new(id, wallet.clone());
            }
            Err(_) => {
                wait();
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(FPT_STAKING_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default().with_salt(salt),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policies)
                .await
                .unwrap();

                return FPTStaking::new(id, wallet.clone());
            }
        }
    }

    pub async fn deploy_usdf_token(wallet: &WalletUnlocked) -> USDFToken<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let id = Contract::load_from(
            &get_absolute_path_from_relative(USDF_TOKEN_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await;

        match id {
            Ok(id) => {
                return USDFToken::new(id, wallet.clone());
            }
            Err(_) => {
                wait();
                let id = Contract::load_from(
                    &get_absolute_path_from_relative(USDF_TOKEN_CONTRACT_BINARY_PATH),
                    LoadConfiguration::default().with_salt(salt),
                )
                .unwrap()
                .deploy(&wallet.clone(), tx_policies)
                .await
                .unwrap();

                return USDFToken::new(id, wallet.clone());
            }
        }
    }

    pub async fn deploy_hint_helper(wallet: &WalletUnlocked) -> HintHelper<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(HINT_HELPER_CONTRACT_BINARY_PATH),
            LoadConfiguration::default().with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), TxPolicies::default().with_tip(1))
        .await
        .unwrap();

        HintHelper::new(id, wallet.clone())
    }

    pub fn print_response<T>(response: &CallResponse<T>)
    where
        T: std::fmt::Debug,
    {
        response.receipts.iter().for_each(|r| match r.ra() {
            Some(r) => println!("{:?}", r),
            _ => (),
        });
    }

    pub fn assert_within_threshold(a: u64, b: u64, comment: &str) {
        let threshold = a / 100000;
        assert!(
            a >= b.saturating_sub(threshold) && a <= b.saturating_add(threshold),
            "{}",
            comment
        );
    }

    pub fn wait() {
        std::thread::sleep(std::time::Duration::from_secs(12));
    }
}
