use super::interfaces::{
    active_pool::{ActivePool, ActivePoolConfigurables},
    borrow_operations::{BorrowOperations, BorrowOperationsConfigurables},
    coll_surplus_pool::{CollSurplusPool, CollSurplusPoolConfigurables},
    community_issuance::{CommunityIssuance, CommunityIssuanceConfigurables},
    default_pool::{DefaultPool, DefaultPoolConfigurables},
    fpt_staking::{FPTStaking, FPTStakingConfigurables},
    fpt_token::{FPTToken, FPTTokenConfigurables},
    hint_helper::HintHelper,
    multi_trove_getter::{MultiTroveGetter, MultiTroveGetterConfigurables},
    oracle::{Oracle, OracleConfigurables},
    protocol_manager::{ProtocolManager, ProtocolManagerConfigurables},
    pyth_oracle::{Price, PythCore, DEFAULT_PYTH_PRICE_ID, PYTH_TIMESTAMP},
    redstone_oracle::{RedstoneCore, DEFAULT_REDSTONE_PRICE_ID},
    sorted_troves::{SortedTroves, SortedTrovesConfigurables},
    stability_pool::{StabilityPool, StabilityPoolConfigurables},
    token::Token,
    trove_manager::{TroveManagerContract, TroveManagerContractConfigurables},
    usdf_token::{USDFToken, USDFTokenConfigurables},
    vesting::{VestingContract, VestingContractConfigurables},
};
use fuels::prelude::{Contract, TxPolicies, WalletUnlocked};

pub mod common {
    use super::*;
    use crate::{
        data_structures::{
            AssetContracts, AssetContractsOptionalRedstone, ContractInstance,
            ExistingAssetContracts, ProtocolContracts, RedstoneConfig, PRECISION,
        },
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
            proxy::Proxy,
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
        tx::StorageSlot,
        types::{Bits256, Bytes32, ContractId, Identity, U256},
    };
    use pbr::ProgressBar;
    // use pbr::ProgressBar;
    use rand::Rng;
    use std::{env, str::FromStr};

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

        let mut contracts = deploy_core_contracts(&wallet, use_test_fpt, false).await;
        initialize_core_contracts(&mut contracts, &wallet, use_test_fpt, true, false).await;

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
        verbose: bool,
    ) -> ProtocolContracts<WalletUnlocked> {
        println!("Deploying core contracts...");
        let mut pb = ProgressBar::new(13);

        let borrow_operations = deploy_borrow_operations(wallet).await;

        if verbose {
            pb.inc();
        }
        let usdf = deploy_usdf_token(wallet).await;
        if verbose {
            pb.inc();
        }
        let stability_pool = deploy_stability_pool(wallet).await;
        if verbose {
            pb.inc();
        }
        let fpt_staking = deploy_fpt_staking(wallet).await;
        if verbose {
            pb.inc();
        }
        let community_issuance = deploy_community_issuance(wallet).await;
        if verbose {
            pb.inc();
        }
        let fpt_token = if use_test_fpt {
            deploy_test_fpt_token(wallet).await
        } else {
            deploy_fpt_token(wallet).await
        };
        if verbose {
            pb.inc();
        }
        let protocol_manager = deploy_protocol_manager(wallet).await;
        if verbose {
            pb.inc();
        }
        let coll_surplus_pool = deploy_coll_surplus_pool(wallet).await;
        if verbose {
            pb.inc();
        }
        let default_pool = deploy_default_pool(wallet).await;
        if verbose {
            pb.inc();
        }
        let active_pool = deploy_active_pool(wallet).await;
        if verbose {
            pb.inc();
        }
        let sorted_troves = deploy_sorted_troves(wallet).await;
        let vesting_contract = deploy_vesting_contract(wallet, 68_000_000 * PRECISION).await;

        let fpt_asset_id = fpt_token
            .contract
            .contract_id()
            .asset_id(&AssetId::zeroed().into());
        if verbose {
            pb.inc();
        }
        let usdf_asset_id = usdf
            .contract
            .contract_id()
            .asset_id(&AssetId::zeroed().into());
        if verbose {
            pb.inc();
        }

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
        verbose: bool,
    ) {
        println!("Initializing core contracts...");
        // contracts.print_contract_ids();
        let mut pb = ProgressBar::new(11);
        if !use_test_fpt {
            fpt_token_abi::initialize(
                &contracts.fpt_token,
                &contracts.vesting_contract.contract,
                &contracts.community_issuance,
            )
            .await;
        }
        if verbose {
            pb.inc();
        }

        community_issuance_abi::initialize(
            &contracts.community_issuance,
            contracts.stability_pool.contract.contract_id().into(),
            contracts.fpt_asset_id,
            &Identity::Address(wallet.address().into()),
            debug,
        )
        .await
        .unwrap();
        if verbose {
            pb.inc();
        }

        usdf_token_abi::initialize(
            &contracts.usdf,
            contracts.protocol_manager.contract.contract_id().into(),
            Identity::ContractId(contracts.stability_pool.contract.contract_id().into()),
            Identity::ContractId(contracts.borrow_operations.contract.contract_id().into()),
        )
        .await
        .unwrap();
        if verbose {
            pb.inc();
        }

        borrow_operations_abi::initialize(
            &contracts.borrow_operations,
            contracts.usdf.contract.contract_id().into(),
            contracts.fpt_staking.contract.contract_id().into(),
            contracts.protocol_manager.contract.contract_id().into(),
            contracts.coll_surplus_pool.contract.contract_id().into(),
            contracts.active_pool.contract.contract_id().into(),
            contracts.sorted_troves.contract.contract_id().into(),
        )
        .await;
        if verbose {
            pb.inc();
        }

        stability_pool_abi::initialize(
            &contracts.stability_pool,
            contracts.usdf.contract.contract_id().into(),
            contracts.community_issuance.contract.contract_id().into(),
            contracts.protocol_manager.contract.contract_id().into(),
            contracts.active_pool.contract.contract_id().into(),
            contracts.sorted_troves.contract.contract_id().into(),
        )
        .await
        .unwrap();
        if verbose {
            pb.inc();
        }

        fpt_staking_abi::initialize(
            &contracts.fpt_staking,
            contracts.protocol_manager.contract.contract_id().into(),
            contracts.borrow_operations.contract.contract_id().into(),
            contracts.fpt_asset_id,
            contracts.usdf_asset_id,
        )
        .await;
        if verbose {
            pb.inc();
        }

        protocol_manager_abi::initialize(
            &contracts.protocol_manager,
            contracts.borrow_operations.contract.contract_id().into(),
            contracts.stability_pool.contract.contract_id().into(),
            contracts.fpt_staking.contract.contract_id().into(),
            contracts.usdf.contract.contract_id().into(),
            contracts.coll_surplus_pool.contract.contract_id().into(),
            contracts.default_pool.contract.contract_id().into(),
            contracts.active_pool.contract.contract_id().into(),
            contracts.sorted_troves.contract.contract_id().into(),
            Identity::Address(wallet.address().into()),
        )
        .await;
        if verbose {
            pb.inc();
        }

        coll_surplus_pool_abi::initialize(
            &contracts.coll_surplus_pool,
            contracts.borrow_operations.contract.contract_id().into(),
            Identity::ContractId(contracts.protocol_manager.contract.contract_id().into()),
        )
        .await
        .unwrap();
        if verbose {
            pb.inc();
        }

        default_pool_abi::initialize(
            &contracts.default_pool,
            Identity::ContractId(contracts.protocol_manager.contract.contract_id().into()),
            contracts.active_pool.contract.contract_id().into(),
        )
        .await
        .unwrap();
        if verbose {
            pb.inc();
        }

        active_pool_abi::initialize(
            &contracts.active_pool,
            Identity::ContractId(contracts.borrow_operations.contract.contract_id().into()),
            Identity::ContractId(contracts.stability_pool.contract.contract_id().into()),
            contracts.default_pool.contract.contract_id().into(),
            Identity::ContractId(contracts.protocol_manager.contract.contract_id().into()),
        )
        .await
        .unwrap();
        if verbose {
            pb.inc();
        }

        sorted_troves_abi::initialize(
            &contracts.sorted_troves,
            100_000_000,
            contracts.protocol_manager.contract.contract_id().into(),
            contracts.borrow_operations.contract.contract_id().into(),
        )
        .await
        .unwrap();
        if verbose {
            pb.inc();
        }
    }

    async fn deploy_test_fpt_token(
        wallet: &WalletUnlocked,
    ) -> ContractInstance<FPTToken<WalletUnlocked>> {
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

        ContractInstance::new(
            FPTToken::new(mock_fpt_token.contract_id().clone(), wallet.clone()),
            mock_fpt_token.contract_id().into(),
        )
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

    pub async fn deploy_fpt_token(
        wallet: &WalletUnlocked,
    ) -> ContractInstance<FPTToken<WalletUnlocked>> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let initializer = Identity::Address(wallet.address().into());
        let configurables = FPTTokenConfigurables::default()
            .with_INITIALIZER(initializer)
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(FPT_TOKEN_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_configurables(configurables.clone())
                .with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await
        .unwrap();

        let proxy = deploy_proxy(
            id.clone().into(),
            wallet.clone(),
            Some(FPT_TOKEN_CONTRACT_STORAGE_PATH),
        )
        .await;

        ContractInstance::new(
            FPTToken::new(proxy.contract_id(), wallet.clone()),
            id.into(),
        )
    }

    pub async fn deploy_sorted_troves(
        wallet: &WalletUnlocked,
    ) -> ContractInstance<SortedTroves<WalletUnlocked>> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let initializer = Identity::Address(wallet.address().into());
        let configurables = SortedTrovesConfigurables::default()
            .with_INITIALIZER(initializer)
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(SORTED_TROVES_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_configurables(configurables.clone())
                .with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await
        .unwrap();

        let proxy = deploy_proxy(
            id.clone().into(),
            wallet.clone(),
            Some(SORTED_TROVES_CONTRACT_STORAGE_PATH),
        )
        .await;

        ContractInstance::new(
            SortedTroves::new(proxy.contract_id(), wallet.clone()),
            id.into(),
        )
    }

    pub async fn deploy_trove_manager_contract(
        wallet: &WalletUnlocked,
    ) -> ContractInstance<TroveManagerContract<WalletUnlocked>> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let initializer = Identity::Address(wallet.address().into());
        let configurables = TroveManagerContractConfigurables::default()
            .with_INITIALIZER(initializer)
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(TROVE_MANAGER_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_configurables(configurables.clone())
                .with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await
        .unwrap();

        let proxy = deploy_proxy(
            id.clone().into(),
            wallet.clone(),
            Some(TROVE_MANAGER_CONTRACT_STORAGE_PATH),
        )
        .await;

        ContractInstance::new(
            TroveManagerContract::new(proxy.contract_id(), wallet.clone()),
            id.into(),
        )
    }

    pub async fn deploy_vesting_contract(
        wallet: &WalletUnlocked,
        total_amount: u64,
    ) -> ContractInstance<VestingContract<WalletUnlocked>> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let initializer = Identity::Address(wallet.address().into());
        let configurables = VestingContractConfigurables::default()
            .with_INITIALIZER(initializer)
            .unwrap()
            .with_TOTAL_AMOUNT(total_amount)
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(VESTING_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_configurables(configurables.clone())
                .with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await
        .unwrap();

        let proxy = deploy_proxy(
            id.clone().into(),
            wallet.clone(),
            Some(VESTING_CONTRACT_STORAGE_PATH),
        )
        .await;

        ContractInstance::new(
            VestingContract::new(proxy.contract_id(), wallet.clone()),
            id.into(),
        )
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
        pyth_price_id: Bits256,
        fuel_vm_decimals: u32,
        debug: bool,
        initializer: Identity,
    ) -> ContractInstance<Oracle<WalletUnlocked>> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let configurables = OracleConfigurables::default()
            .with_PYTH(pyth)
            .unwrap()
            .with_PYTH_PRICE_ID(pyth_price_id)
            .unwrap()
            .with_DEBUG(debug)
            .unwrap()
            .with_FUEL_DECIMAL_REPRESENTATION(fuel_vm_decimals)
            .unwrap()
            .with_INITIALIZER(initializer)
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(ORACLE_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_salt(salt)
                .with_configurables(configurables.clone()),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await
        .unwrap();

        let proxy = deploy_proxy(
            id.clone().into(),
            wallet.clone(),
            Some(ORACLE_CONTRACT_STORAGE_PATH),
        )
        .await;

        ContractInstance::new(Oracle::new(proxy.contract_id(), wallet.clone()), id.into())
    }

    pub async fn deploy_protocol_manager(
        wallet: &WalletUnlocked,
    ) -> ContractInstance<ProtocolManager<WalletUnlocked>> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let initializer = Identity::Address(wallet.address().into());
        let configurables = ProtocolManagerConfigurables::default()
            .with_INITIALIZER(initializer)
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(PROTCOL_MANAGER_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_salt(salt)
                .with_configurables(configurables.clone()),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await
        .unwrap();

        let proxy = deploy_proxy(
            id.clone().into(),
            wallet.clone(),
            Some(PROTCOL_MANAGER_CONTRACT_STORAGE_PATH),
        )
        .await;

        ContractInstance::new(
            ProtocolManager::new(proxy.contract_id(), wallet.clone()),
            id.into(),
        )
    }

    pub async fn deploy_borrow_operations(
        wallet: &WalletUnlocked,
    ) -> ContractInstance<BorrowOperations<WalletUnlocked>> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let initializer = Identity::Address(wallet.address().into());
        let configurables = BorrowOperationsConfigurables::default()
            .with_INITIALIZER(initializer)
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(BORROW_OPERATIONS_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_configurables(configurables.clone())
                .with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await
        .unwrap();

        let proxy = deploy_proxy(
            id.clone().into(),
            wallet.clone(),
            Some(BORROW_OPERATIONS_CONTRACT_STORAGE_PATH),
        )
        .await;

        ContractInstance::new(
            BorrowOperations::new(proxy.contract_id(), wallet.clone()),
            id.into(),
        )
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
        existing_contracts: &ExistingAssetContracts,
        debug: bool,
        deploy_redstone: bool,
    ) -> AssetContractsOptionalRedstone<WalletUnlocked> {
        println!("Deploying asset contracts...");
        let mut pb = ProgressBar::new(6);

        pb.inc();

        let (asset, asset_id, fuel_vm_decimals) = match &existing_contracts.asset {
            Some(asset_contract) => {
                pb.inc();
                (
                    asset_contract.asset,
                    asset_contract.asset_id,
                    asset_contract.fuel_vm_decimals,
                )
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

                println!("Deploying asset contracts... Done");
                println!("Asset: {}", asset.contract_id());

                let _ = token_abi::initialize(
                    &asset,
                    1_000_000_000,
                    &Identity::Address(wallet.address().into()),
                    "MOCK".to_string(),
                    "MOCK".to_string(),
                )
                .await
                .unwrap();
                (asset.contract_id().into(), asset_id, 9) // Default fuel_vm_decimals to 9
            }
        };

        // Deploy or use existing Pyth oracle
        let (mock_pyth_oracle, pyth_price_id) = match &existing_contracts.pyth_oracle {
            Some(pyth_config) => {
                pb.inc();
                (
                    PythCore::new(pyth_config.contract, wallet.clone()),
                    pyth_config.price_id,
                )
            }
            None => {
                let pyth = deploy_mock_pyth_oracle(&wallet).await;
                let pyth_price_id = Bits256::from(asset_id);
                pb.inc();
                let pyth_feed = vec![(
                    pyth_price_id,
                    Price {
                        confidence: 0,
                        exponent: 9,
                        price: 1 * PRECISION,
                        publish_time: PYTH_TIMESTAMP,
                    },
                )];
                pyth_oracle_abi::update_price_feeds(&pyth, pyth_feed).await;
                (pyth, pyth_price_id)
            }
        };

        // Deploy or use existing Redstone oracle
        let redstone_config: Option<RedstoneConfig> = match &existing_contracts.redstone_oracle {
            Some(redstone_config) => {
                pb.inc();
                Some(RedstoneConfig {
                    contract: redstone_config.contract.into(),
                    price_id: redstone_config.price_id,
                    precision: redstone_config.precision,
                })
            }
            None => {
                if deploy_redstone {
                    let redstone = deploy_mock_redstone_oracle(&wallet).await;
                    let redstone_price_id = U256::from(rand::thread_rng().gen_range(1..1_000_000));
                    let redstone_feed = redstone_price_feed_with_id(redstone_price_id, vec![1]);
                    redstone_oracle_abi::write_prices(&redstone, redstone_feed).await;
                    redstone_oracle_abi::set_timestamp(&redstone, PYTH_TIMESTAMP).await;
                    pb.inc();
                    Some(RedstoneConfig {
                        contract: redstone.contract_id().into(),
                        price_id: redstone_price_id,
                        precision: 9,
                    })
                } else {
                    None
                }
            }
        };

        // Always deploy a new oracle and trove manager
        let oracle = deploy_oracle(
            &wallet,
            mock_pyth_oracle.contract_id().into(),
            pyth_price_id,
            fuel_vm_decimals,
            debug,
            Identity::Address(wallet.address().into()),
        )
        .await;
        pb.inc();

        let trove_manager = deploy_trove_manager_contract(&wallet).await;
        pb.inc();

        // Set up price feeds if we deployed new oracles
        if debug {
            oracle_abi::set_debug_timestamp(&oracle, PYTH_TIMESTAMP).await;
        }

        println!("Deploying asset contracts... Done");
        println!("Oracle: {}", oracle.contract.contract_id());
        println!("Mock Pyth Oracle: {}", mock_pyth_oracle.contract_id());

        println!("Trove Manager: {}", trove_manager.contract.contract_id());
        println!("Asset: {}", asset);
        println!("Asset ID: {}", asset_id);
        println!("Pyth Price ID: {:?}", pyth_price_id);
        match &redstone_config {
            Some(redstone_config) => {
                println!("Redstone Oracle: {}", redstone_config.contract);
                println!("Redstone Price ID: {}", redstone_config.price_id);
                println!("Redstone Precision: {}", redstone_config.precision);
            }
            None => println!("No Redstone Oracle"),
        }
        println!("Fuel VM Decimals: {}", fuel_vm_decimals);

        AssetContractsOptionalRedstone {
            symbol: existing_contracts.symbol.clone(),
            oracle,
            mock_pyth_oracle,
            redstone_config,
            trove_manager,
            asset: Token::new(asset, wallet.clone()),
            asset_id,
            pyth_price_id,
            fuel_vm_decimals,
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
            DEFAULT_PYTH_PRICE_ID,
            9,
            true,
            Identity::Address(wallet.address().into()),
        )
        .await;
        let trove_manager = deploy_trove_manager_contract(wallet).await;
        let asset = deploy_token(wallet).await;

        token_abi::initialize(
            &asset,
            1_000_000_000,
            &Identity::Address(wallet.address().into()),
            name,
            symbol.clone(),
        )
        .await
        .unwrap();

        trove_manager_abi::initialize(
            &trove_manager,
            contracts.borrow_operations.contract.contract_id().into(),
            contracts.sorted_troves.contract.contract_id().into(),
            oracle.contract.contract_id().into(),
            contracts.stability_pool.contract.contract_id().into(),
            contracts.default_pool.contract.contract_id().into(),
            contracts.active_pool.contract.contract_id().into(),
            contracts.coll_surplus_pool.contract.contract_id().into(),
            contracts.usdf.contract.contract_id().into(),
            asset
                .contract_id()
                .asset_id(&AssetId::zeroed().into())
                .into(),
            contracts.protocol_manager.contract.contract_id().into(),
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
            trove_manager.contract.contract_id().into(),
            oracle.contract.contract_id().into(),
            &contracts.borrow_operations,
            &contracts.stability_pool,
            &contracts.usdf,
            &contracts.fpt_staking,
            &contracts.coll_surplus_pool,
            &contracts.default_pool,
            &contracts.active_pool,
            &contracts.sorted_troves,
        )
        .await
        .unwrap();

        let asset_id: AssetId = asset
            .contract_id()
            .asset_id(&AssetId::zeroed().into())
            .into();

        AssetContracts {
            oracle,
            mock_pyth_oracle: pyth,
            mock_redstone_oracle: redstone,
            trove_manager,
            asset,
            asset_id,
            pyth_price_id: DEFAULT_PYTH_PRICE_ID,
            redstone_price_id: DEFAULT_REDSTONE_PRICE_ID,
            redstone_precision: 9,
            fuel_vm_decimals: 9,
            symbol,
        }
    }

    pub async fn initialize_asset<T: Account>(
        core_protocol_contracts: &ProtocolContracts<T>,
        asset_contracts: &AssetContractsOptionalRedstone<T>,
    ) -> Result<CallResponse<()>> {
        println!("Initializing asset contracts...");
        let mut pb = ProgressBar::new(2);

        let _ = trove_manager_abi::initialize(
            &asset_contracts.trove_manager,
            core_protocol_contracts
                .borrow_operations
                .contract
                .contract_id()
                .into(),
            core_protocol_contracts
                .sorted_troves
                .contract
                .contract_id()
                .into(),
            asset_contracts.oracle.contract.contract_id().into(),
            core_protocol_contracts
                .stability_pool
                .contract
                .contract_id()
                .into(),
            core_protocol_contracts
                .default_pool
                .contract
                .contract_id()
                .into(),
            core_protocol_contracts
                .active_pool
                .contract
                .contract_id()
                .into(),
            core_protocol_contracts
                .coll_surplus_pool
                .contract
                .contract_id()
                .into(),
            core_protocol_contracts.usdf.contract.contract_id().into(),
            asset_contracts.asset_id,
            core_protocol_contracts
                .protocol_manager
                .contract
                .contract_id()
                .into(),
        )
        .await
        .unwrap();
        pb.inc();

        protocol_manager_abi::register_asset(
            &core_protocol_contracts.protocol_manager,
            asset_contracts.asset_id,
            asset_contracts.trove_manager.contract.contract_id().into(),
            asset_contracts.oracle.contract.contract_id().into(),
            &core_protocol_contracts.borrow_operations,
            &core_protocol_contracts.stability_pool,
            &core_protocol_contracts.usdf,
            &core_protocol_contracts.fpt_staking,
            &core_protocol_contracts.coll_surplus_pool,
            &core_protocol_contracts.default_pool,
            &core_protocol_contracts.active_pool,
            &core_protocol_contracts.sorted_troves,
        )
        .await
    }

    pub async fn deploy_active_pool(
        wallet: &WalletUnlocked,
    ) -> ContractInstance<ActivePool<WalletUnlocked>> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let initializer = Identity::Address(wallet.address().into());
        let configurables = ActivePoolConfigurables::default()
            .with_INITIALIZER(initializer)
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(ACTIVE_POOL_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_configurables(configurables.clone())
                .with_salt(salt),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await
        .unwrap();

        let proxy = deploy_proxy(
            id.clone().into(),
            wallet.clone(),
            Some(ACTIVE_POOL_CONTRACT_STORAGE_PATH),
        )
        .await;

        ContractInstance::new(
            ActivePool::new(proxy.contract_id(), wallet.clone()),
            id.into(),
        )
    }

    pub async fn deploy_stability_pool(
        wallet: &WalletUnlocked,
    ) -> ContractInstance<StabilityPool<WalletUnlocked>> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let initializer = Identity::Address(wallet.address().into());
        let configurables = StabilityPoolConfigurables::default()
            .with_INITIALIZER(initializer)
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(STABILITY_POOL_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_salt(salt)
                .with_configurables(configurables.clone()),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await
        .unwrap();

        let proxy = deploy_proxy(
            id.clone().into(),
            wallet.clone(),
            Some(STABILITY_POOL_CONTRACT_STORAGE_PATH),
        )
        .await;

        ContractInstance::new(
            StabilityPool::new(proxy.contract_id(), wallet.clone()),
            id.into(),
        )
    }

    pub async fn deploy_default_pool(
        wallet: &WalletUnlocked,
    ) -> ContractInstance<DefaultPool<WalletUnlocked>> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let initializer = Identity::Address(wallet.address().into());
        let configurables = DefaultPoolConfigurables::default()
            .with_INITIALIZER(initializer)
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(DEFAULT_POOL_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_salt(salt)
                .with_configurables(configurables.clone()),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await
        .unwrap();

        let proxy = deploy_proxy(
            id.clone().into(),
            wallet.clone(),
            Some(DEFAULT_POOL_CONTRACT_STORAGE_PATH),
        )
        .await;

        ContractInstance::new(
            DefaultPool::new(proxy.contract_id(), wallet.clone()),
            id.into(),
        )
    }

    pub async fn deploy_coll_surplus_pool(
        wallet: &WalletUnlocked,
    ) -> ContractInstance<CollSurplusPool<WalletUnlocked>> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let initializer = Identity::Address(wallet.address().into());
        let configurables = CollSurplusPoolConfigurables::default()
            .with_INITIALIZER(initializer)
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(COLL_SURPLUS_POOL_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_salt(salt)
                .with_configurables(configurables.clone()),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await
        .unwrap();

        let proxy = deploy_proxy(
            id.clone().into(),
            wallet.clone(),
            Some(COLL_SURPLUS_POOL_CONTRACT_STORAGE_PATH),
        )
        .await;

        ContractInstance::new(
            CollSurplusPool::new(proxy.contract_id(), wallet.clone()),
            id.into(),
        )
    }

    pub async fn deploy_community_issuance(
        wallet: &WalletUnlocked,
    ) -> ContractInstance<CommunityIssuance<WalletUnlocked>> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let initializer = Identity::Address(wallet.address().into());
        let configurables = CommunityIssuanceConfigurables::default()
            .with_INITIALIZER(initializer)
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(COMMUNITY_ISSUANCE_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_salt(salt)
                .with_configurables(configurables.clone()),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await
        .unwrap();

        let proxy = deploy_proxy(
            id.clone().into(),
            wallet.clone(),
            Some(COMMUNITY_ISSUANCE_CONTRACT_STORAGE_PATH),
        )
        .await;

        ContractInstance::new(
            CommunityIssuance::new(proxy.contract_id(), wallet.clone()),
            id.into(),
        )
    }

    pub async fn deploy_fpt_staking(
        wallet: &WalletUnlocked,
    ) -> ContractInstance<FPTStaking<WalletUnlocked>> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let initializer = Identity::Address(wallet.address().into());
        let configurables = FPTStakingConfigurables::default()
            .with_INITIALIZER(initializer)
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(FPT_STAKING_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_salt(salt)
                .with_configurables(configurables.clone()),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await
        .unwrap();

        let proxy = deploy_proxy(
            id.clone().into(),
            wallet.clone(),
            Some(FPT_STAKING_CONTRACT_STORAGE_PATH),
        )
        .await;

        ContractInstance::new(
            FPTStaking::new(proxy.contract_id(), wallet.clone()),
            id.into(),
        )
    }

    pub async fn deploy_usdf_token(
        wallet: &WalletUnlocked,
    ) -> ContractInstance<USDFToken<WalletUnlocked>> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();
        let tx_policies = TxPolicies::default().with_tip(1);

        let initializer = Identity::Address(wallet.address().into());
        let configurables = USDFTokenConfigurables::default()
            .with_INITIALIZER(initializer)
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(USDF_TOKEN_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_salt(salt)
                .with_configurables(configurables.clone()),
        )
        .unwrap()
        .deploy(&wallet.clone(), tx_policies)
        .await
        .unwrap();

        let proxy = deploy_proxy(
            id.clone().into(),
            wallet.clone(),
            Some(USDF_TOKEN_CONTRACT_STORAGE_PATH),
        )
        .await;

        ContractInstance::new(
            USDFToken::new(proxy.contract_id(), wallet.clone()),
            id.into(),
        )
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

    pub async fn deploy_multi_trove_getter(
        wallet: &WalletUnlocked,
        sorted_troves_contract_id: &ContractId,
    ) -> MultiTroveGetter<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();

        let configurables = MultiTroveGetterConfigurables::default()
            .with_SORTED_TROVES_CONTRACT(sorted_troves_contract_id.clone().into())
            .unwrap();

        let id = Contract::load_from(
            &get_absolute_path_from_relative(MULTI_TROVE_GETTER_CONTRACT_BINARY_PATH),
            LoadConfiguration::default()
                .with_salt(salt)
                .with_configurables(configurables.clone()),
        )
        .unwrap()
        .deploy(&wallet.clone(), TxPolicies::default().with_tip(1))
        .await
        .unwrap();

        MultiTroveGetter::new(id, wallet.clone())
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

    pub async fn deploy_proxy(
        target: ContractId,
        owner: WalletUnlocked,
        additional_storage_path: Option<&str>,
    ) -> Proxy<WalletUnlocked> {
        let mut rng = rand::thread_rng();
        let salt = rng.gen::<[u8; 32]>();

        // Storage keys for the proxy target contract
        // These match the storage slots defined in the proxy contract's storage layout
        // See contracts/proxy-contract/src/main.sw storage section
        let target_key0 =
            Bytes32::from_str("0x7bb458adc1d118713319a5baa00a2d049dd64d2916477d2688d76970c898cd55")
                .unwrap();
        let target_key1 =
            Bytes32::from_str("0x7bb458adc1d118713319a5baa00a2d049dd64d2916477d2688d76970c898cd56")
                .unwrap();

        // Convert target ContractId to storage value format
        let target_value = Bytes32::new(target.into());
        let mut target_value0 = Bytes32::new([0u8; 32]);
        let mut target_value1 = Bytes32::new([0u8; 32]);

        // Split target value across two storage slots
        // First slot: Set flag byte and first part of target
        target_value0[7] = 1; // Flag byte indicating initialized state
        for n in 8..32 {
            target_value0[n] = target_value[n - 8];
        }
        // Second slot: Remaining bytes of target
        for n in 0..8 {
            target_value1[n] = target_value[n + 24];
        }

        // Storage keys for the proxy owner
        // These match the storage slots for the proxy owner state
        let owner_key0 =
            Bytes32::from_str("bb79927b15d9259ea316f2ecb2297d6cc8851888a98278c0a2e03e1a091ea754")
                .unwrap();
        let owner_key1 =
            Bytes32::from_str("bb79927b15d9259ea316f2ecb2297d6cc8851888a98278c0a2e03e1a091ea755")
                .unwrap();

        // Convert owner address to storage value format
        let owner_value = Bytes32::new(Address::from(owner.address()).into());
        let mut owner_value0 = Bytes32::new([0u8; 32]);
        let mut owner_value1 = Bytes32::new([0u8; 32]);

        // Split owner value across two storage slots
        // First slot: Set flag byte and first part of owner address
        owner_value0[7] = 1; // Flag byte indicating initialized state
        for n in 16..32 {
            owner_value0[n] = owner_value[n - 16];
        }
        // Second slot: Remaining bytes of owner address
        for n in 0..16 {
            owner_value1[n] = owner_value[n + 16];
        }

        // Create storage configuration with the initialized slots
        let storage_slots = [
            StorageSlot::new(target_key0, target_value0),
            StorageSlot::new(target_key1, target_value1),
            StorageSlot::new(owner_key0, owner_value0),
            StorageSlot::new(owner_key1, owner_value1),
        ];
        let storage_configuration = match additional_storage_path {
            Some(path) => StorageConfiguration::default()
                .add_slot_overrides(storage_slots)
                .add_slot_overrides_from_file(get_absolute_path_from_relative(path))
                .unwrap(),
            None => StorageConfiguration::default().add_slot_overrides(storage_slots),
        };
        // Deploy the proxy contract with the initialized storage
        let contract_configuration =
            LoadConfiguration::default().with_storage_configuration(storage_configuration);

        let contract_id = Contract::load_from(
            &get_absolute_path_from_relative(PROXY_CONTRACT_BINARY_PATH),
            contract_configuration,
        )
        .unwrap()
        .with_salt(salt)
        .deploy(&owner, TxPolicies::default())
        .await
        .unwrap();

        Proxy::new(contract_id, owner)
    }
}
