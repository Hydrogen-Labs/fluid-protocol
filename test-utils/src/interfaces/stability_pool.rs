use fuels::prelude::abigen;

use crate::interfaces::community_issuance::CommunityIssuance;
use crate::interfaces::oracle::Oracle;
use crate::interfaces::pyth_oracle::PythCore;
use crate::interfaces::redstone_oracle::RedstoneCore;
use crate::interfaces::sorted_troves::SortedTroves;
use crate::interfaces::token::Token;
use crate::interfaces::trove_manager::TroveManagerContract;
use crate::interfaces::usdf_token::USDFToken;

abigen!(Contract(
    name = "StabilityPool",
    abi = "contracts/stability-pool-contract/out/debug/stability-pool-contract-abi.json"
));

pub mod stability_pool_abi {

    use super::*;
    use fuels::{
        prelude::{Account, CallParameters, Error, TxPolicies, WalletUnlocked},
        programs::{calls::ContractDependency, responses::CallResponse},
        types::{transaction_builders::VariableOutputPolicy, AssetId, ContractId, Identity},
    };

    pub async fn initialize<T: Account>(
        stability_pool: &StabilityPool<T>,

        usdf_address: ContractId,
        community_issuance_address: ContractId,
        protocol_manager_contract: ContractId,
        active_pool: ContractId,
        sorted_troves: ContractId,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default().with_tip(1);

        stability_pool
            .methods()
            .initialize(
                usdf_address,
                community_issuance_address,
                protocol_manager_contract,
                active_pool,
                sorted_troves,
            )
            .with_tx_policies(tx_params)
            .call()
            .await
    }

    pub async fn add_asset<T: Account>(
        stability_pool: &StabilityPool<T>,
        trove_manager: ContractId,
        asset_address: AssetId,
        oracle_address: ContractId,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default().with_tip(1);

        stability_pool
            .methods()
            .add_asset(trove_manager, asset_address.into(), oracle_address)
            .with_tx_policies(tx_params)
            .call()
            .await
    }

    pub async fn provide_to_stability_pool<T: Account>(
        stability_pool: &StabilityPool<T>,
        community_issuance: &CommunityIssuance<T>,
        usdf_token: &USDFToken<T>,
        mock_token: &Token<T>,
        amount: u64,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default().with_tip(1);

        let usdf_asset_id = usdf_token.contract_id().asset_id(&AssetId::zeroed().into());

        let call_params: CallParameters = CallParameters::default()
            .with_amount(amount)
            .with_asset_id(usdf_asset_id);

        stability_pool
            .methods()
            .provide_to_stability_pool()
            .with_tx_policies(tx_params)
            .call_params(call_params)
            .unwrap()
            .with_variable_output_policy(VariableOutputPolicy::Exactly(2))
            .with_contracts(&[usdf_token, mock_token, community_issuance])
            .call()
            .await
    }

    pub async fn get_asset<T: Account>(
        stability_pool: &StabilityPool<T>,
        asset_address: AssetId,
    ) -> Result<CallResponse<u64>, Error> {
        stability_pool
            .methods()
            .get_asset(asset_address)
            .call()
            .await
    }

    pub async fn get_total_usdf_deposits<T: Account>(
        stability_pool: &StabilityPool<T>,
    ) -> Result<CallResponse<u64>, Error> {
        stability_pool
            .methods()
            .get_total_usdf_deposits()
            .call()
            .await
    }

    pub async fn get_depositor_asset_gain<T: Account>(
        stability_pool: &StabilityPool<T>,
        depositor: Identity,
        asset_id: AssetId,
    ) -> Result<CallResponse<u64>, Error> {
        stability_pool
            .methods()
            .get_depositor_asset_gain(depositor, asset_id.into())
            .call()
            .await
    }

    pub async fn get_compounded_usdf_deposit(
        stability_pool: &StabilityPool<WalletUnlocked>,
        depositor: Identity,
    ) -> Result<CallResponse<u64>, Error> {
        stability_pool
            .methods()
            .get_compounded_usdf_deposit(depositor)
            .call()
            .await
    }

    pub async fn get_depositor_fpt_gain<T: Account>(
        stability_pool: &StabilityPool<T>,
        depositor: Identity,
    ) -> Result<CallResponse<u64>, Error> {
        stability_pool
            .methods()
            .get_depositor_fpt_gain(depositor)
            .call()
            .await
    }

    pub async fn withdraw_from_stability_pool<T: Account>(
        stability_pool: &StabilityPool<T>,
        community_issuance: &CommunityIssuance<T>,
        usdf_token: &USDFToken<T>,
        mock_token: &Token<T>,
        sorted_troves: &SortedTroves<T>,
        oracle: &Oracle<T>,
        pyth_oracle: &PythCore<T>,
        redstone_oracle: &Option<RedstoneCore<T>>,
        trove_manager: &TroveManagerContract<T>,
        amount: u64,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default().with_tip(1);

        let mut contracts: Vec<&dyn ContractDependency> = vec![
            usdf_token,
            mock_token,
            community_issuance,
            sorted_troves,
            oracle,
            pyth_oracle,
            trove_manager,
        ];

        if let Some(redstone) = redstone_oracle {
            contracts.push(redstone);
        }

        stability_pool
            .methods()
            .withdraw_from_stability_pool(amount)
            .with_tx_policies(tx_params)
            .with_variable_output_policy(VariableOutputPolicy::Exactly(2))
            .with_contracts(&contracts)
            .call()
            .await
    }
}

pub mod stability_pool_utils {
    use fuels::{
        prelude::{Account, WalletUnlocked},
        types::{AssetId, Identity},
    };

    use crate::setup::common::assert_within_threshold;

    use super::*;

    pub async fn assert_pool_asset<T: Account>(
        stability_pool: &StabilityPool<T>,
        expected_asset_amount: u64,
        asset_address: AssetId,
    ) {
        let pool_asset = super::stability_pool_abi::get_asset(stability_pool, asset_address.into())
            .await
            .unwrap()
            .value;

        assert_eq!(pool_asset, expected_asset_amount);
    }

    pub async fn assert_total_usdf_deposits<T: Account>(
        stability_pool: &StabilityPool<T>,
        expected_usdf_amount: u64,
    ) {
        let total_usdf_deposits =
            super::stability_pool_abi::get_total_usdf_deposits(stability_pool)
                .await
                .unwrap()
                .value;

        assert_eq!(total_usdf_deposits, expected_usdf_amount);
    }

    pub async fn assert_depositor_asset_gain<T: Account>(
        stability_pool: &StabilityPool<T>,
        depositor: Identity,
        expected_asset_gain: u64,
        asset_address: AssetId,
    ) {
        let depositor_asset_gain = super::stability_pool_abi::get_depositor_asset_gain(
            stability_pool,
            depositor,
            asset_address,
        )
        .await
        .unwrap()
        .value;

        assert_within_threshold(
            expected_asset_gain,
            depositor_asset_gain,
            &format!(
                "Depsoitor gains not within 0.001% threshold, expected: {}, real: {}",
                expected_asset_gain, depositor_asset_gain
            ),
        );
    }

    pub async fn assert_compounded_usdf_deposit(
        stability_pool: &StabilityPool<WalletUnlocked>,
        depositor: Identity,
        expected_compounded_usdf_deposit: u64,
    ) {
        let compounded_usdf_deposit =
            stability_pool_abi::get_compounded_usdf_deposit(stability_pool, depositor)
                .await
                .unwrap()
                .value;

        assert_within_threshold(
            expected_compounded_usdf_deposit,
            compounded_usdf_deposit,
            &format!(
                "Compounded USDF deposit not within 0.001% threshold, expected: {}, real: {}",
                expected_compounded_usdf_deposit, compounded_usdf_deposit
            ),
        );
    }
}
