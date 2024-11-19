use fuels::prelude::abigen;

use fuels::programs::responses::CallResponse;
abigen!(Contract(
    name = "MultiTroveGetter",
    abi = "contracts/multi-trove-getter-contract/out/debug/multi-trove-getter-contract-abi.json"
));

pub mod multi_trove_getter_abi {
    use super::*;
    use crate::data_structures::ContractInstance;
    use crate::interfaces::{sorted_troves::SortedTroves, trove_manager::TroveManagerContract};
    use fuels::prelude::Account;
    use fuels::types::errors::Error;
    use fuels::types::AssetId;

    pub async fn get_multiple_sorted_troves<T: Account>(
        multi_trove_getter: &MultiTroveGetter<T>,
        trove_manager: &ContractInstance<TroveManagerContract<T>>,
        sorted_troves: &ContractInstance<SortedTroves<T>>,
        asset_id: &AssetId,
        start_indx: u64,
        count: u8,
    ) -> Result<CallResponse<Vec<CombinedTroveData>>, Error> {
        multi_trove_getter
            .methods()
            .get_multiple_sorted_troves(
                trove_manager.contract.contract_id(),
                asset_id.clone(),
                start_indx,
                count,
            )
            .with_contracts(&[&sorted_troves.contract, &trove_manager.contract])
            .with_contract_ids(&[
                sorted_troves.contract.contract_id().into(),
                sorted_troves.implementation_id.into(),
                trove_manager.contract.contract_id().into(),
                trove_manager.implementation_id.into(),
            ])
            .call()
            .await
    }
}

pub mod multi_trove_getter_utils {
    use super::*;
    use crate::data_structures::{ContractInstance, PRECISION};
    use crate::interfaces::sorted_troves::SortedTroves;
    use crate::interfaces::trove_manager::TroveManagerContract;
    use fuels::prelude::Account;
    use fuels::types::AssetId;

    pub async fn print_troves_cr<T: Account>(
        multi_trove_getter: &MultiTroveGetter<T>,
        trove_manager: &ContractInstance<TroveManagerContract<T>>,
        sorted_troves: &ContractInstance<SortedTroves<T>>,
        asset_id: &AssetId,
    ) {
        let troves_after_rewards = multi_trove_getter_abi::get_multiple_sorted_troves(
            multi_trove_getter,
            trove_manager,
            sorted_troves,
            asset_id,
            0,
            10,
        )
        .await
        .unwrap()
        .value;

        for trove in troves_after_rewards {
            println!(
                "CR: {:?}, collateral: {:?}, debt: {:?}, unapplied_coll_rewards: {:?}, unapplied_debt_rewards: {:?}",
                get_trove_cr(&trove),
                trove.collateral,
                trove.debt,
                trove.collateral_rewards,
                trove.debt_rewards
            )
        }
    }

    pub async fn assert_sorted_troves_by_cr<T: Account>(
        multi_trove_getter: &MultiTroveGetter<T>,
        trove_manager: &ContractInstance<TroveManagerContract<T>>,
        sorted_troves: &ContractInstance<SortedTroves<T>>,
        asset_id: &AssetId,
    ) {
        let troves = multi_trove_getter_abi::get_multiple_sorted_troves(
            multi_trove_getter,
            trove_manager,
            sorted_troves,
            asset_id,
            0,
            10,
        )
        .await
        .unwrap()
        .value;

        let len = troves.len();
        if len <= 1 {
            return;
        }

        for i in 0..len.saturating_sub(1) {
            assert!(
                get_trove_cr(&troves[i]) <= get_trove_cr(&troves[i + 1]),
                "Troves are not sorted by CR"
            );
        }
    }

    fn get_trove_cr(trove: &CombinedTroveData) -> u128 {
        (trove.collateral as u128 * PRECISION as u128 / trove.debt as u128) as u128
    }
}
