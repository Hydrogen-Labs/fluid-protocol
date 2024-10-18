use crate::interfaces::active_pool::ActivePool;
use crate::interfaces::coll_surplus_pool::CollSurplusPool;
use crate::interfaces::community_issuance::CommunityIssuance;
use crate::interfaces::default_pool::DefaultPool;
use crate::interfaces::oracle::Oracle;
use crate::interfaces::pyth_oracle::PythCore;
use crate::interfaces::redstone_oracle::RedstoneCore;
use crate::interfaces::sorted_troves::SortedTroves;
use crate::interfaces::stability_pool::StabilityPool;
use crate::interfaces::usdf_token::USDFToken;
use fuels::prelude::abigen;
use fuels::programs::responses::CallResponse;

abigen!(Contract(
    name = "TroveManagerContract",
    abi = "contracts/trove-manager-contract/out/debug/trove-manager-contract-abi.json"
));

pub mod trove_manager_abi {

    use fuels::{
        prelude::{Account, Error, TxPolicies},
        programs::calls::ContractDependency,
        types::{transaction_builders::VariableOutputPolicy, AssetId, ContractId, Identity},
    };

    use super::*;

    pub async fn get_nominal_icr<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        id: Identity,
    ) -> CallResponse<u64> {
        trove_manager
            .methods()
            .get_nominal_icr(id)
            .call()
            .await
            .unwrap()
    }

    pub async fn batch_liquidate_troves<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        community_issuance: &CommunityIssuance<T>,
        stability_pool: &StabilityPool<T>,
        oracle: &Oracle<T>,
        pyth: &PythCore<T>,
        redstone: &Option<RedstoneCore<T>>,
        sorted_troves: &SortedTroves<T>,
        active_pool: &ActivePool<T>,
        default_pool: &DefaultPool<T>,
        coll_surplus_pool: &CollSurplusPool<T>,
        usdf: &USDFToken<T>,
        ids: Vec<Identity>,
        upper_hint: Identity,
        lower_hint: Identity,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_script_gas_limit(2000000);

        let mut contracts: Vec<&dyn ContractDependency> = vec![
            stability_pool,
            oracle,
            pyth,
            sorted_troves,
            active_pool,
            default_pool,
            coll_surplus_pool,
            usdf,
            community_issuance,
        ];

        if let Some(redstone) = redstone {
            contracts.push(redstone);
        }

        trove_manager
            .methods()
            .batch_liquidate_troves(ids, upper_hint, lower_hint)
            .with_tx_policies(tx_params)
            .with_contracts(&contracts)
            .with_variable_output_policy(VariableOutputPolicy::Exactly(3))
            .call()
            .await
    }

    pub async fn liquidate<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        community_issuance: &CommunityIssuance<T>,
        stability_pool: &StabilityPool<T>,
        oracle: &Oracle<T>,
        pyth: &PythCore<T>,
        redstone: &Option<RedstoneCore<T>>,
        sorted_troves: &SortedTroves<T>,
        active_pool: &ActivePool<T>,
        default_pool: &DefaultPool<T>,
        coll_surplus_pool: &CollSurplusPool<T>,
        usdf: &USDFToken<T>,
        id: Identity,
        upper_hint: Identity,
        lower_hint: Identity,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default().with_tip(1);

        let mut contracts: Vec<&dyn ContractDependency> = vec![
            stability_pool,
            oracle,
            pyth,
            sorted_troves,
            active_pool,
            default_pool,
            coll_surplus_pool,
            usdf,
            community_issuance,
        ];

        if let Some(redstone) = redstone {
            contracts.push(redstone);
        }

        trove_manager
            .methods()
            .liquidate(id, upper_hint, lower_hint)
            .with_tx_policies(tx_params)
            .with_contracts(&contracts)
            .with_variable_output_policy(VariableOutputPolicy::Exactly(3))
            .call()
            .await
    }

    pub async fn increase_trove_coll<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        id: Identity,
        amount: u64,
    ) -> CallResponse<u64> {
        let tx_params = TxPolicies::default().with_tip(1);

        trove_manager
            .methods()
            .increase_trove_coll(id, amount)
            .with_tx_policies(tx_params)
            .call()
            .await
            .unwrap()
    }

    pub async fn increase_trove_debt<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        id: Identity,
        amount: u64,
    ) -> CallResponse<u64> {
        let tx_params = TxPolicies::default().with_tip(1);

        trove_manager
            .methods()
            .increase_trove_debt(id, amount)
            .with_tx_policies(tx_params)
            .call()
            .await
            .unwrap()
    }

    pub async fn set_trove_status<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        id: Identity,
        status: Status,
    ) -> CallResponse<()> {
        let tx_params = TxPolicies::default().with_tip(1);

        trove_manager
            .methods()
            .set_trove_status(id, status)
            .with_tx_policies(tx_params)
            .call()
            .await
            .unwrap()
    }

    pub async fn initialize<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        borrow_operations: ContractId,
        sorted_troves_id: ContractId,
        oracle_id: ContractId,
        stability_pool: ContractId,
        default_pool: ContractId,
        active_pool: ContractId,
        coll_surplus_pool: ContractId,
        usdf: ContractId,
        asset: AssetId,
        protocol_manager: ContractId,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default().with_tip(1);

        let res = trove_manager
            .methods()
            .initialize(
                borrow_operations,
                sorted_troves_id,
                oracle_id,
                stability_pool,
                default_pool,
                active_pool,
                coll_surplus_pool,
                usdf,
                asset.into(),
                protocol_manager,
            )
            .with_tx_policies(tx_params)
            .call()
            .await;

        return res;
    }

    pub async fn get_trove_coll<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        id: Identity,
    ) -> CallResponse<u64> {
        let tx_params = TxPolicies::default().with_tip(1);

        trove_manager
            .methods()
            .get_trove_coll(id)
            .with_tx_policies(tx_params)
            .call()
            .await
            .unwrap()
    }

    pub async fn get_entire_debt_and_coll<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        id: Identity,
    ) -> CallResponse<(u64, u64, u64, u64)> {
        let tx_params = TxPolicies::default().with_tip(1);

        trove_manager
            .methods()
            .get_entire_debt_and_coll(id)
            .with_tx_policies(tx_params)
            .call()
            .await
            .unwrap()
    }

    pub async fn get_trove_debt<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        id: Identity,
    ) -> CallResponse<u64> {
        trove_manager
            .methods()
            .get_trove_debt(id)
            .call()
            .await
            .unwrap()
    }

    pub async fn get_trove_status<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        id: Identity,
    ) -> Result<CallResponse<Status>, Error> {
        trove_manager.methods().get_trove_status(id).call().await
    }

    pub async fn get_pending_asset_reward<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        id: Identity,
    ) -> CallResponse<u64> {
        trove_manager
            .methods()
            .get_pending_asset_rewards(id)
            .call()
            .await
            .unwrap()
    }

    pub async fn get_pending_usdf_reward<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        id: Identity,
    ) -> CallResponse<u64> {
        trove_manager
            .methods()
            .get_pending_usdf_rewards(id)
            .call()
            .await
            .unwrap()
    }

    pub fn get_redemption_fee(asset_drawdown: u64) -> u64 {
        return asset_drawdown * 1 / 100;
    }
}

pub mod trove_manager_utils {
    use fuels::{
        prelude::Account,
        types::{AssetId, Identity},
    };

    use crate::{
        interfaces::sorted_troves::sorted_troves_abi, setup::common::assert_within_threshold,
    };

    use super::*;

    pub async fn set_coll_and_debt_insert<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        sorted_troves: &SortedTroves<T>,
        id: Identity,
        coll: u64,
        debt: u64,
        prev_id: Identity,
        next_id: Identity,
        asset: AssetId,
    ) {
        trove_manager_abi::increase_trove_coll(trove_manager, id.clone(), coll).await;
        trove_manager_abi::increase_trove_debt(trove_manager, id.clone(), debt).await;
        trove_manager_abi::set_trove_status(trove_manager, id.clone(), Status::Active).await;
        sorted_troves_abi::insert(sorted_troves, id, coll, prev_id, next_id, asset).await;
    }

    pub async fn assert_trove_coll<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        id: Identity,
        expected_coll: u64,
    ) {
        let real_coll = trove_manager_abi::get_trove_coll(&trove_manager, id)
            .await
            .value;

        assert_eq!(real_coll, expected_coll);
    }

    pub async fn assert_trove_debt<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        id: Identity,
        expected_debt: u64,
    ) {
        let real_debt = trove_manager_abi::get_trove_debt(&trove_manager, id)
            .await
            .value;

        assert_eq!(real_debt, expected_debt, "Incorrect trove debt");
    }

    pub async fn assert_trove_status<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        id: Identity,
        expected_status: Status,
    ) {
        let real_status = trove_manager_abi::get_trove_status(&trove_manager, id)
            .await
            .unwrap()
            .value;

        assert_eq!(real_status, expected_status, "Incorrect trove status");
    }

    pub async fn assert_pending_asset_rewards<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        id: Identity,
        expected_rewards: u64,
    ) {
        let real_rewards = trove_manager_abi::get_pending_asset_reward(&trove_manager, id)
            .await
            .value;

        assert_within_threshold(
            real_rewards,
            expected_rewards,
            &format!(
                "Rewards are not within 0.001% threshold, expected: {}, real: {}",
                expected_rewards, real_rewards
            ),
        );
    }

    pub async fn assert_pending_usdf_rewards<T: Account>(
        trove_manager: &TroveManagerContract<T>,
        id: Identity,
        expected_rewards: u64,
    ) {
        let real_rewards = trove_manager_abi::get_pending_usdf_reward(&trove_manager, id)
            .await
            .value;

        assert_within_threshold(
            real_rewards,
            expected_rewards,
            &format!(
                "USDF Rewards are not within 0.001% threshold, expected: {}, real: {}",
                expected_rewards, real_rewards
            ),
        );
    }
}
