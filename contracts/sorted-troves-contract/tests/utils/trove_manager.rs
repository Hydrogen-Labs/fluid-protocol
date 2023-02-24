use fuels::prelude::*;
use fuels::programs::call_response::FuelCallResponse;
use fuels::types::Identity;

use crate::utils::setup::TroveManagerContract;
use test_utils::interfaces::sorted_troves::SortedTroves;

pub mod trove_manager_abi_calls {

    use super::*;

    pub async fn set_nominal_icr_and_insert(
        trove_manager: &TroveManagerContract,
        sorted_troves: &SortedTroves,
        new_id: Identity,
        new_icr: u64,
        prev_id: Identity,
        next_id: Identity,
    ) -> FuelCallResponse<()> {
        let tx_params = TxParameters::new(Some(1), Some(100_000_000), Some(0));

        trove_manager
            .methods()
            .set_nominal_icr_and_insert(new_id, new_icr, prev_id, next_id)
            .set_contracts(&[sorted_troves])
            .tx_params(tx_params)
            .call()
            .await
            .unwrap()
    }

    pub async fn get_nominal_icr(
        trove_manager: &TroveManagerContract,
        id: Identity,
    ) -> FuelCallResponse<u64> {
        trove_manager
            .methods()
            .get_nominal_icr(id)
            .call()
            .await
            .unwrap()
    }

    pub async fn remove(
        trove_manager: &TroveManagerContract,
        sorted_troves: &SortedTroves,
        id: Identity,
    ) -> FuelCallResponse<()> {
        let tx_params = TxParameters::new(Some(1), Some(100_000_000), Some(0));

        trove_manager
            .methods()
            .remove(id)
            .set_contracts(&[sorted_troves])
            .tx_params(tx_params)
            .call()
            .await
            .unwrap()
    }
}

pub async fn deploy_trove_manager_contract(wallet: &WalletUnlocked) -> TroveManagerContract {
    let id = Contract::deploy(
        &get_path("../trove-manager-contract/out/debug/trove-manager-contract.bin".to_string()),
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_storage_path(Some(get_path(
            "../trove-manager-contract/out/debug/trove-manager-contract-storage_slots.json"
                .to_string(),
        ))),
    )
    .await
    .unwrap();

    TroveManagerContract::new(id, wallet.clone())
}

fn get_path(mut sub_path: String) -> String {
    let mut path = std::env::current_dir().unwrap();
    // if sub_path starts with ../, we need to go up one level
    if sub_path.starts_with("../") {
        path.pop();

        // remove the ../ from the sub_path
        sub_path = sub_path[3..].to_string();
    }
    path.push(sub_path);
    path.to_str().unwrap().to_string()
}
