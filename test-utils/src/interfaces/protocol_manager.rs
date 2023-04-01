use fuels::prelude::abigen;

use fuels::programs::call_response::FuelCallResponse;

abigen!(Contract(
    name = "ProtocolManager",
    abi = "contracts/protocol-manager-contract/out/debug/protocol-manager-contract-abi.json"
));

pub mod protocol_manager_abi {
    use crate::interfaces::active_pool::ActivePool;
    use crate::interfaces::borrow_operations::BorrowOperations;
    use crate::interfaces::coll_surplus_pool::CollSurplusPool;
    use crate::interfaces::default_pool::DefaultPool;
    use crate::interfaces::oracle::Oracle;
    use crate::interfaces::sorted_troves::SortedTroves;
    use crate::interfaces::stability_pool::StabilityPool;
    use crate::interfaces::token::Token;
    use crate::interfaces::trove_manager::TroveManagerContract;
    use crate::interfaces::usdf_token::USDFToken;
    use crate::setup::common::wait;
    use fuels::prelude::{CallParameters, LogDecoder};
    use fuels::types::AssetId;
    use fuels::{
        prelude::{ContractId, TxParameters},
        types::Identity,
    };

    use super::*;

    pub async fn initialize(
        protocol_manager: &ProtocolManager,
        borrow_operations: ContractId,
        stability_pool: ContractId,
        usdf: ContractId,
        admin: Identity,
    ) -> FuelCallResponse<()> {
        let tx_params = TxParameters::default().set_gas_price(1);

        let res = protocol_manager
            .methods()
            .initialize(borrow_operations, stability_pool, usdf, admin)
            .tx_params(tx_params)
            .call()
            .await;

        // TODO: remove this workaround
        match res {
            Ok(res) => res,
            Err(_) => {
                wait();
                return FuelCallResponse::new((), vec![], LogDecoder::default());
            }
        }
    }

    pub async fn register_asset(
        protocol_manager: &ProtocolManager,
        asset: ContractId,
        active_pool: ContractId,
        trove_manager: ContractId,
        coll_surplus_pool: ContractId,
        oracle: ContractId,
        sorted_troves: ContractId,
        borrow_operations: &BorrowOperations,
        stability_pool: &StabilityPool,
        usdf: &USDFToken,
    ) -> FuelCallResponse<()> {
        let tx_params = TxParameters::default().set_gas_price(1);

        protocol_manager
            .methods()
            .register_asset(
                asset,
                active_pool,
                trove_manager,
                coll_surplus_pool,
                oracle,
                sorted_troves,
            )
            .tx_params(tx_params)
            .set_contracts(&[borrow_operations, stability_pool, usdf])
            .call()
            .await
            .unwrap()
    }

    pub async fn redeem_collateral(
        protocol_manager: &ProtocolManager,
        amount: u64,
        max_iterations: u64,
        max_fee_percentage: u64,
        partial_redemption_hint: u64,
        upper_partial_hint: Option<Identity>,
        lower_partial_hint: Option<Identity>,
        usdf: &USDFToken,
        fuel: &Token,
        sorted_troves: &SortedTroves,
        active_pool: &ActivePool,
        coll_surplus_pool: &CollSurplusPool,
        oracle: &Oracle,
        default_pool: &DefaultPool,
        trove_manager: &TroveManagerContract,
    ) -> FuelCallResponse<()> {
        let tx_params = TxParameters::default()
            .set_gas_price(1)
            .set_gas_limit(2000000);
        let usdf_asset_id = AssetId::from(*usdf.contract_id().hash());

        let call_params: CallParameters = CallParameters::default()
            .set_amount(amount)
            .set_asset_id(usdf_asset_id);

        protocol_manager
            .methods()
            .redeem_collateral(
                max_iterations,
                max_fee_percentage,
                partial_redemption_hint,
                upper_partial_hint.unwrap_or(Identity::Address([0; 32].into())),
                lower_partial_hint.unwrap_or(Identity::Address([0; 32].into())),
            )
            .tx_params(tx_params)
            .call_params(call_params)
            .unwrap()
            .set_contracts(&[
                sorted_troves,
                active_pool,
                fuel,
                usdf,
                coll_surplus_pool,
                oracle,
                default_pool,
                trove_manager,
            ])
            .append_variable_outputs(10)
            .call()
            .await
            .unwrap()
    }
}
