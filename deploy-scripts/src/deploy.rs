use std::{fs::File, io::Write};

use dotenv::dotenv;
use fuels::prelude::*;
use serde_json::json;

pub mod deployment {
    const VESTING_SCHEDULE_PATH: &str = "deploy-scripts/vesting/test_vesting.json";
    use test_utils::data_structures::ProtocolContracts;
    use test_utils::interfaces::vesting::{self, load_vesting_schedules_from_json_file};

    use crate::utils::utils::setup_wallet;

    use super::*;

    use test_utils::setup::common::{deploy_core_contracts, initialize_core_contracts};

    pub async fn deploy() {
        //--------------- Deploy ---------------
        dotenv().ok();

        //--------------- WALLET ---------------
        let wallet = setup_wallet().await;
        let address = wallet.address();
        println!("🔑 Wallet address: {}", address);

        //--------------- Deploy ---------------
        let core_contracts = deploy_and_initialize_all_core_contracts(wallet).await;

        //--------------- Write to file ---------------
        write_contracts_to_file(core_contracts)
    }

    pub async fn deploy_and_initialize_all_core_contracts(
        wallet: WalletUnlocked,
    ) -> ProtocolContracts<WalletUnlocked> {
        let vesting_schedules = load_vesting_schedules_from_json_file(VESTING_SCHEDULE_PATH);

        let mut core_contracts = deploy_core_contracts(&wallet, false).await;
        initialize_core_contracts(&mut core_contracts, &wallet, false, false).await;

        let _ = vesting::instantiate_vesting_contract(
            &core_contracts.vesting_contract,
            &core_contracts.fpt_asset_id,
            vesting_schedules,
        )
        .await;

        return core_contracts;
    }

    fn write_contracts_to_file(contracts: ProtocolContracts<WalletUnlocked>) {
        let mut file = File::create("contracts.json").unwrap();

        let json = json!({
            "borrow_operations": contracts.borrow_operations.contract_id().to_string(),
            "usdf": contracts.usdf.contract_id().to_string(),
            "usdf_asset_id": contracts.usdf.contract_id().asset_id(&AssetId::zeroed().into()).to_string(),
            "stability_pool": contracts.stability_pool.contract_id().to_string(),
            "protocol_manager": contracts.protocol_manager.contract_id().to_string(),
            "fpt_staking": contracts.fpt_staking.contract_id().to_string(),
            "fpt_token": contracts.fpt_token.contract_id().to_string(),
            "fpt_asset_id": contracts.fpt_asset_id.to_string(),
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

    pub fn wait() {
        // Necessary for random instances where the 'UTXO' cannot be found
        std::thread::sleep(std::time::Duration::from_secs(15));
    }
}
