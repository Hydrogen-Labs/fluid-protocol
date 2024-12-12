use std::{fs::File, io::Write};

use dotenv::dotenv;
use fuels::prelude::*;
use fuels::types::{Bits256, Identity};
use serde_json::json;
use std::str::FromStr;
use test_utils::data_structures::ProtocolContracts;
use test_utils::interfaces::hint_helper::HintHelper;
use test_utils::interfaces::multi_trove_getter::MultiTroveGetter;
use test_utils::interfaces::vesting;

use crate::constants::{MAINNET_CONTRACTS_FILE, TESTNET_CONTRACTS_FILE};
use crate::utils::utils::{is_testnet, load_vesting_schedules_from_csv, setup_wallet};

use test_utils::setup::common::{
    deploy_core_contracts, deploy_hint_helper, deploy_multi_trove_getter, initialize_core_contracts,
};

const VESTING_SCHEDULE_PATH: &str = "deploy-scripts/vesting/vesting.csv";
const CLIFF_PERCENTAGE: f64 = 0.0; // 0% cliff
const SECONDS_TO_CLIFF: u64 = 7 * 24 * 60 * 60; // 7 days
const SECONDS_VESTING_DURATION: u64 = 2 * 365 * 24 * 60 * 60; // 2 years

pub mod deployment {
    use crate::constants::{MAINNET_TREASURY_IDENTITY, TESTNET_TREASURY_IDENTITY};

    use super::*;
    pub async fn deploy() {
        //--------------- Deploy ---------------
        dotenv().ok();

        let wallet = setup_wallet().await;
        let network_name = wallet.provider().unwrap().chain_info().await.unwrap().name;
        let address: Address = wallet.address().into();
        let is_testnet = is_testnet(wallet.clone()).await;
        //--------------- WALLET ---------------
        println!("🔑 Wallet address: 0x{}", address);
        println!("🔑 Is testnet: {}", is_testnet);
        println!("🔑 Network name: {}", network_name);
        println!(
            "🔑 Treasury identity: {}",
            match is_testnet {
                true => TESTNET_TREASURY_IDENTITY,
                false => MAINNET_TREASURY_IDENTITY,
            }
        );
        //--------------- Deploy ---------------
        let core_contracts =
            deploy_and_initialize_all_core_contracts(wallet.clone(), is_testnet).await;
        let (hint_helper, multi_trove_getter) =
            deploy_frontend_helper_contracts(wallet.clone(), &core_contracts).await;

        //--------------- Write to file ---------------
        write_contracts_to_file(core_contracts, hint_helper, multi_trove_getter, is_testnet);
    }

    pub async fn deploy_and_initialize_all_core_contracts(
        wallet: WalletUnlocked,
        is_testnet: bool,
    ) -> ProtocolContracts<WalletUnlocked> {
        let treasury_identity = Identity::Address(
            Address::from_str(match is_testnet {
                true => TESTNET_TREASURY_IDENTITY,
                false => MAINNET_TREASURY_IDENTITY,
            })
            .unwrap(),
        );

        let vesting_schedules = load_vesting_schedules_from_csv(
            VESTING_SCHEDULE_PATH,
            CLIFF_PERCENTAGE,
            SECONDS_TO_CLIFF,
            SECONDS_VESTING_DURATION,
            treasury_identity,
        );
        let mut core_contracts = deploy_core_contracts(&wallet, false, true).await;
        initialize_core_contracts(&mut core_contracts, &wallet, false, false, true).await;

        vesting::vesting_abi::instantiate_vesting_contract(
            &core_contracts.vesting_contract,
            &core_contracts.fpt_asset_id,
            vesting_schedules,
            false,
        )
        .await
        .unwrap();

        return core_contracts;
    }

    pub async fn deploy_frontend_helper_contracts(
        wallet: WalletUnlocked,
        core_contracts: &ProtocolContracts<WalletUnlocked>,
    ) -> (HintHelper<WalletUnlocked>, MultiTroveGetter<WalletUnlocked>) {
        let hint_helper = deploy_hint_helper(&wallet).await;
        let multi_trove_getter = deploy_multi_trove_getter(
            &wallet,
            &core_contracts.sorted_troves.contract.contract_id().into(),
        )
        .await;

        return (hint_helper, multi_trove_getter);
    }

    pub fn write_contracts_to_file(
        contracts: ProtocolContracts<WalletUnlocked>,
        hint_helper: HintHelper<WalletUnlocked>,
        multi_trove_getter: MultiTroveGetter<WalletUnlocked>,
        is_testnet: bool,
    ) {
        let mut file = File::create(match is_testnet {
            true => TESTNET_CONTRACTS_FILE,
            false => MAINNET_CONTRACTS_FILE,
        })
        .unwrap();

        let json = json!({
            "borrow_operations": contracts.borrow_operations.contract.contract_id().to_string(),
            "borrow_operations_implementation_id": format!("0x{}", contracts.borrow_operations.implementation_id.to_string()),
            "usdf": contracts.usdf.contract.contract_id().to_string(),
            "usdf_implementation_id": format!("0x{}", contracts.usdf.implementation_id.to_string()),
            "usdf_asset_id": format!("0x{}", contracts.usdf_asset_id.to_string()),
            "stability_pool": contracts.stability_pool.contract.contract_id().to_string(),
            "stability_pool_implementation_id": format!("0x{}", contracts.stability_pool.implementation_id.to_string()),
            "protocol_manager": contracts.protocol_manager.contract.contract_id().to_string(),
            "protocol_manager_implementation_id": format!("0x{}", contracts.protocol_manager.implementation_id.to_string()),
            "fpt_staking": contracts.fpt_staking.contract.contract_id().to_string(),
            "fpt_staking_implementation_id": format!("0x{}", contracts.fpt_staking.implementation_id.to_string()),
            "fpt_token": contracts.fpt_token.contract.contract_id().to_string(),
            "fpt_token_implementation_id": format!("0x{}", contracts.fpt_token.implementation_id.to_string()),
            "fpt_asset_id": format!("0x{}", contracts.fpt_asset_id.to_string()),
            "community_issuance": contracts.community_issuance.contract.contract_id().to_string(),
            "community_issuance_implementation_id": format!("0x{}", contracts.community_issuance.implementation_id.to_string()),
            "coll_surplus_pool": contracts.coll_surplus_pool.contract.contract_id().to_string(),
            "coll_surplus_pool_implementation_id": format!("0x{}", contracts.coll_surplus_pool.implementation_id.to_string()),
            "default_pool": contracts.default_pool.contract.contract_id().to_string(),
            "default_pool_implementation_id": format!("0x{}", contracts.default_pool.implementation_id.to_string()),
            "active_pool": contracts.active_pool.contract.contract_id().to_string(),
            "active_pool_implementation_id": format!("0x{}", contracts.active_pool.implementation_id.to_string()),
            "sorted_troves": contracts.sorted_troves.contract.contract_id().to_string(),
            "sorted_troves_implementation_id": format!("0x{}", contracts.sorted_troves.implementation_id.to_string()),
            "vesting_contract": contracts.vesting_contract.contract.contract_id().to_string(),
            "vesting_contract_implementation_id": format!("0x{}", contracts.vesting_contract.implementation_id.to_string()),
            "hint_helper": hint_helper.contract_id().to_string(),
            "multi_trove_getter": multi_trove_getter.contract_id().to_string(),
            "asset_contracts": contracts.asset_contracts.iter().map(|asset_contracts| {
                json!({
                    "oracle": asset_contracts.oracle.contract.contract_id().to_string(),
                    "oracle_implementation_id": format!("0x{}", asset_contracts.oracle.implementation_id.to_string()),
                    "trove_manager": asset_contracts.trove_manager.contract.contract_id().to_string(),
                    "trove_manager_implementation_id": format!("0x{}", asset_contracts.trove_manager.implementation_id.to_string()),
                    "asset_contract": asset_contracts.asset.contract_id().to_string(),
                    "asset_id": format!("0x{}", asset_contracts.asset_id.to_string()),
                })
            }).collect::<Vec<serde_json::Value>>()
        });

        file.write_all(serde_json::to_string_pretty(&json).unwrap().as_bytes())
            .unwrap();
    }

    pub fn wait() {
        // Necessary for random instances where the 'UTXO' cannot be found
        std::thread::sleep(std::time::Duration::from_secs(15));
    }

    pub fn to_hex_str(bits: &Bits256) -> String {
        format!("0x{}", hex::encode(bits.0))
    }
}
