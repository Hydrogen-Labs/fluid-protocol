use super::interfaces::{
    active_pool::ActivePool,
    borrow_operations::BorrowOperations,
    coll_surplus_pool::CollSurplusPool,
    community_issuance::CommunityIssuance,
    default_pool::DefaultPool,
    fpt_staking::FPTStaking,
    fpt_token::FPTToken,
    hint_helper::HintHelper,
    oracle::{Oracle, OracleConfigurables, ORACLE_TIMEOUT},
    protocol_manager::ProtocolManager,
    pyth_oracle::{PythCore, PYTH_PRICE_ID},
    redstone_oracle::{RedstoneCore, REDSTONE_PRICE_ID},
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
        interfaces::{
            active_pool::active_pool_abi,
            borrow_operations::borrow_operations_abi,
            coll_surplus_pool::coll_surplus_pool_abi,
            community_issuance::community_issuance_abi,
            default_pool::default_pool_abi,
            fpt_staking::fpt_staking_abi,
            fpt_token::fpt_token_abi,
            protocol_manager::protocol_manager_abi,
            pyth_oracle::{pyth_oracle_abi, pyth_price_feed},
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
        types::{ContractId, Identity},
    };
    use pbr::ProgressBar;
    use rand::Rng;
    use std::env;

    pub struct ProtocolContracts<T: Account> {
        pub borrow_operations: BorrowOperations<T>,
        pub usdf: USDFToken<T>,
        pub stability_pool: StabilityPool<T>,
        pub protocol_manager: ProtocolManager<T>,
        pub asset_contracts: Vec<AssetContracts<T>>,
        pub fpt_staking: FPTStaking<T>,
        pub coll_surplus_pool: CollSurplusPool<T>,
        pub sorted_troves: SortedTroves<T>,
        pub default_pool: DefaultPool<T>,
        pub active_pool: ActivePool<T>,
        pub fpt_token: FPTToken<T>,
        pub community_issuance: CommunityIssuance<T>,
        pub vesting_contract: VestingContract<T>,
    }

    pub struct AssetContracts<T: Account> {
        pub asset: Token<T>,
        pub oracle: Oracle<T>,
        pub mock_pyth_oracle: PythCore<T>,
        pub mock_redstone_oracle: RedstoneCore<T>,
        pub trove_manager: TroveManagerContract<T>,
        pub asset_id: AssetId,
    }

    pub struct ExistingAssetContracts {
        pub asset: ContractId,
        pub oracle: ContractId,
        pub pyth_oracle: ContractId,
        pub redstone_oracle: ContractId,
    }

    pub async fn setup_protocol(
        max_size: u64,
        num_wallets: u64,
        deploy_2nd_asset: bool,
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

        let contracts =
            deploy_and_initialize_all(wallet.clone(), max_size, false, deploy_2nd_asset).await;

        (contracts, wallet, wallets)
    }

    pub async fn deploy_and_initialize_all(
        wallet: WalletUnlocked,
        _max_size: u64,
        is_testnet: bool,
        deploy_2nd_asset: bool,
    ) -> ProtocolContracts<WalletUnlocked> {
        println!("Deploying parent contracts...");
        // let mut pb = ProgressBar::new(12);

        let borrow_operations = deploy_borrow_operations(&wallet).await;
        // pb.inc();

        let usdf = deploy_usdf_token(&wallet).await;
        // pb.inc();

        let stability_pool = deploy_stability_pool(&wallet).await;
        // pb.inc();

        let fpt_staking = deploy_fpt_staking(&wallet).await;
        // pb.inc();

        let community_issuance = deploy_community_issuance(&wallet).await;
        // pb.inc();

        let fpt_token = deploy_fpt_token(&wallet).await;
        // pb.inc();

        let protocol_manager = deploy_protocol_manager(&wallet).await;
        // pb.inc();

        let coll_surplus_pool = deploy_coll_surplus_pool(&wallet).await;
        // pb.inc();

        let default_pool = deploy_default_pool(&wallet).await;
        // pb.inc();

        let active_pool = deploy_active_pool(&wallet).await;
        // pb.inc();

        let sorted_troves = deploy_sorted_troves(&wallet).await;

        let vesting_contract = deploy_vesting_contract(&wallet).await;
        // pb.inc();

        // pb.finish_println("Parent Contracts deployed");

        if is_testnet {
            println!("Borrow operations: {}", borrow_operations.contract_id());
            println!("Usdf: {}", usdf.contract_id());
            println!("Stability Pool: {}", stability_pool.contract_id());
            println!("Protocol Manager: {}", protocol_manager.contract_id());
            println!("FPT Staking: {}", fpt_staking.contract_id());
            println!("FPT Token: {}", fpt_token.contract_id());
            println!("Community Issuance: {}", community_issuance.contract_id());
            println!("Coll Surplus Pool: {}", coll_surplus_pool.contract_id());
            println!("Default Pool: {}", default_pool.contract_id());
        }

        // let mut pb = ProgressBar::new(12);

        let mut asset_contracts: Vec<AssetContracts<WalletUnlocked>> = vec![];

        community_issuance_abi::initialize(
            &community_issuance,
            stability_pool.contract_id().into(),
            fpt_token
                .contract_id()
                .asset_id(&AssetId::zeroed().into())
                .into(),
            &Identity::Address(wallet.address().into()),
            true,
        )
        .await
        .unwrap();
        // pb.inc();

        fpt_token_abi::initialize(&fpt_token, &vesting_contract, &community_issuance, true).await;
        // pb.inc();

        usdf_token_abi::initialize(
            &usdf,
            protocol_manager.contract_id().into(),
            Identity::ContractId(stability_pool.contract_id().into()),
            Identity::ContractId(borrow_operations.contract_id().into()),
        )
        .await
        .unwrap();
        // pb.inc();

        borrow_operations_abi::initialize(
            &borrow_operations,
            usdf.contract_id().into(),
            fpt_staking.contract_id().into(),
            protocol_manager.contract_id().into(),
            coll_surplus_pool.contract_id().into(),
            active_pool.contract_id().into(),
            sorted_troves.contract_id().into(),
        )
        .await;
        // pb.inc();

        stability_pool_abi::initialize(
            &stability_pool,
            usdf.contract_id().into(),
            community_issuance.contract_id().into(),
            protocol_manager.contract_id().into(),
            active_pool.contract_id().into(),
        )
        .await
        .unwrap();
        // pb.inc();

        
        fpt_staking_abi::initialize(
            &fpt_staking,
            protocol_manager.contract_id().into(),
            borrow_operations.contract_id().into(),
            fpt_token.contract_id().asset_id(&AssetId::zeroed().into()).into(), // TODO switch this from `fpt` to `fpt_token`, mock token for testing
            usdf.contract_id()
                .asset_id(&AssetId::zeroed().into())
                .into(),
        )
        .await;
        // pb.inc();

        protocol_manager_abi::initialize(
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
        // pb.inc();

        coll_surplus_pool_abi::initialize(
            &coll_surplus_pool,
            borrow_operations.contract_id().into(),
            Identity::ContractId(protocol_manager.contract_id().into()),
        )
        .await
        .unwrap();
        // pb.inc();

        default_pool_abi::initialize(
            &default_pool,
            Identity::ContractId(protocol_manager.contract_id().into()),
            active_pool.contract_id().into(),
        )
        .await
        .unwrap();
        // pb.inc();

        active_pool_abi::initialize(
            &active_pool,
            Identity::ContractId(borrow_operations.contract_id().into()),
            Identity::ContractId(stability_pool.contract_id().into()),
            default_pool.contract_id().into(),
            Identity::ContractId(protocol_manager.contract_id().into()),
        )
        .await
        .unwrap();
        // pb.inc();

        sorted_troves_abi::initialize(
            &sorted_troves,
            100,
            protocol_manager.contract_id().into(),
            borrow_operations.contract_id().into(),
        )
        .await
        .unwrap();
        // pb.inc();

        let fuel_asset_contracts = add_asset(
            &borrow_operations,
            &stability_pool,
            &protocol_manager,
            &usdf,
            &fpt_staking,
            &coll_surplus_pool,
            &default_pool,
            &active_pool,
            &sorted_troves,
            wallet.clone(),
            "Fuel".to_string(),
            "FUEL".to_string(),
            is_testnet,
        )
        .await;

        if deploy_2nd_asset {
            let usdf_asset_contracts = add_asset(
                &borrow_operations,
                &stability_pool,
                &protocol_manager,
                &usdf,
                &fpt_staking,
                &coll_surplus_pool,
                &default_pool,
                &active_pool,
                &sorted_troves,
                wallet,
                "stFuel".to_string(),
                "stFUEL".to_string(),
                is_testnet,
            )
            .await;
            // pb.finish();

            asset_contracts.push(usdf_asset_contracts);
        }
        // pb.finish();

        asset_contracts.push(fuel_asset_contracts);

        let contracts = ProtocolContracts {
            borrow_operations,
            usdf,
            stability_pool,
            asset_contracts,
            protocol_manager,
            fpt_staking,
            fpt_token,
            coll_surplus_pool,
            default_pool,
            active_pool,
            sorted_troves,
            community_issuance,
            vesting_contract,
        };

        return contracts;
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
            Ok(id) => return Token::new(id, wallet.clone()),
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

                return Token::new(id, wallet.clone());
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
        redstone: ContractId,
        redstone_precison: u8,
    ) -> Oracle<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let configurables = OracleConfigurables::default()
            .with_PYTH(pyth)
            .unwrap()
            .with_PYTH_PRICE_ID(PYTH_PRICE_ID)
            .unwrap()
            .with_REDSTONE(redstone)
            .unwrap()
            .with_REDSTONE_PRICE_ID(REDSTONE_PRICE_ID)
            .unwrap()
            .with_TIMEOUT(ORACLE_TIMEOUT)
            .unwrap()
            .with_DEBUG(true)
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

    pub async fn add_asset(
        borrow_operations: &BorrowOperations<WalletUnlocked>,
        stability_pool: &StabilityPool<WalletUnlocked>,
        protocol_manager: &ProtocolManager<WalletUnlocked>,
        usdf: &USDFToken<WalletUnlocked>,
        fpt_staking: &FPTStaking<WalletUnlocked>,
        coll_surplus_pool: &CollSurplusPool<WalletUnlocked>,
        default_pool: &DefaultPool<WalletUnlocked>,
        active_pool: &ActivePool<WalletUnlocked>,
        sorted_troves: &SortedTroves<WalletUnlocked>,
        wallet: WalletUnlocked,
        name: String,
        symbol: String,
        is_testnet: bool,
    ) -> AssetContracts<WalletUnlocked> {
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
        let trove_manager = deploy_trove_manager_contract(&wallet).await;
        let asset = deploy_token(&wallet).await;

        if is_testnet {
            println!("Deployed asset: {}", asset.contract_id());
            println!("Deployed trove manager: {}", trove_manager.contract_id());
            println!("Deployed oracle: {}", oracle.contract_id());
            println!("Deployed mock pyth oracle: {}", pyth.contract_id());
            println!("Deployed mock redstone oracle: {}", redstone.contract_id());
            println!("Deployed fpt staking: {}", fpt_staking.contract_id());
        }

        token_abi::initialize(
            &asset,
            1_000_000_000,
            &Identity::Address(wallet.address().into()),
            name.to_string(),
            symbol.to_string(),
        )
        .await
        .unwrap();

        trove_manager_abi::initialize(
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
        .await
        .unwrap();

        pyth_oracle_abi::update_price_feeds(&pyth, pyth_price_feed(1)).await;

        protocol_manager_abi::register_asset(
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
            &coll_surplus_pool,
            default_pool,
            active_pool,
            &sorted_troves,
        )
        .await;

        let asset_id: AssetId = asset
            .contract_id()
            .asset_id(&AssetId::zeroed().into())
            .into();

        return AssetContracts {
            oracle,
            mock_pyth_oracle: pyth,
            mock_redstone_oracle: redstone,
            trove_manager,
            asset,
            asset_id,
        };
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
