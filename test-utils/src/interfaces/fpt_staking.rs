use fuels::prelude::{abigen, TxParameters};

use fuels::programs::call_response::FuelCallResponse;

abigen!(Contract(
    name = "FPTStaking",
    abi = "contracts/fpt-staking-contract/out/debug/fpt-staking-contract-abi.json"
));

pub mod fpt_staking_abi {

    use crate::interfaces::token::Token;
    use crate::interfaces::usdf_token::USDFToken;
    use fuels::prelude::{Account, AssetId, CallParameters, Error};
    use fuels::{prelude::ContractId, types::Identity};
    // use crate::setup::common::wait;

    use super::*;

    pub async fn initialize<T: Account>(
        fpt_staking: &FPTStaking<T>,
        protocol_manager_address: ContractId,
        trove_manager_address: ContractId,
        borrower_operations_address: ContractId,
        fpt_address: ContractId,
        usdf_address: ContractId,
    ) -> FuelCallResponse<()> {
        // let tx_params = TxParameters::default().set_gas_price(1);

        fpt_staking
            .methods()
            .initialize(
                protocol_manager_address,
                trove_manager_address,
                borrower_operations_address,
                fpt_address,
                usdf_address,
            )
            .call()
            .await
            .unwrap()
    }

    pub async fn get_storage<T: Account>(
        fpt_staking: &FPTStaking<T>,
    ) -> FuelCallResponse<fpt_staking_abi::ReadStorage> {
        let tx_params = TxParameters::default().set_gas_price(1);

        fpt_staking
            .methods()
            .get_storage()
            .tx_params(tx_params)
            .call()
            .await
            .unwrap()
    }

    pub async fn stake<T: Account>(
        fpt_staking: &FPTStaking<T>,
        fpt_token: &Token<T>,
        fpt_deposit_amount: u64,
    ) -> FuelCallResponse<()> {
        let tx_params = TxParameters::default().set_gas_price(1);

        let fpt_asset_id = AssetId::from(*fpt_token.contract_id().hash());

        let call_params: CallParameters = CallParameters::default()
            .set_amount(fpt_deposit_amount)
            .set_asset_id(fpt_asset_id);

        fpt_staking
            .methods()
            .stake()
            .tx_params(tx_params)
            .call_params(call_params)
            .unwrap()
            .call()
            .await
            .unwrap()
    }

    pub async fn unstake<T: Account>(
        fpt_staking: &FPTStaking<T>,
        usdf_token: &USDFToken<T>,
        fuel_token: &Token<T>,
        fpt_token: &Token<T>,
        amount: u64,
    ) -> Result<FuelCallResponse<()>, Error> {
        let tx_params = TxParameters::default()
            .set_gas_price(1)
            .set_gas_limit(2000000);

        fpt_staking
            .methods()
            .unstake(amount)
            .tx_params(tx_params)
            .set_contracts(&[usdf_token, fuel_token, fpt_token])
            .append_variable_outputs(10)
            .call()
            .await
    }

    pub async fn add_asset<T: Account>(
        fpt_staking: &FPTStaking<T>,
        asset_address: ContractId,
    ) -> FuelCallResponse<()> {
        // let tx_params = TxParameters::default().set_gas_price(1);

        fpt_staking
            .methods()
            .add_asset(asset_address)
            .call()
            .await
            .unwrap()
    }

    pub async fn get_pending_asset_gain<T: Account>(
        fpt_staking: &FPTStaking<T>,
        id: Identity,
        asset_address: ContractId,
    ) -> FuelCallResponse<u64> {
        // let tx_params = TxParameters::default().set_gas_price(1);

        fpt_staking
            .methods()
            .get_pending_asset_gain(id, asset_address)
            .call()
            .await
            .unwrap()
    }

    pub async fn get_pending_usdf_gain<T: Account>(
        fpt_staking: &FPTStaking<T>,
        id: Identity,
    ) -> FuelCallResponse<u64> {
        // let tx_params = TxParameters::default().set_gas_price(1);

        fpt_staking
            .methods()
            .get_pending_usdf_gain(id)
            .call()
            .await
            .unwrap()
    }

    pub async fn increase_f_usdf<T: Account>(
        fpt_staking: &FPTStaking<T>,
        usdf_fee_amount: u64,
    ) -> FuelCallResponse<()> {
        // let tx_params = TxParameters::default().set_gas_price(1);

        fpt_staking
            .methods()
            .increase_f_usdf(usdf_fee_amount)
            .call()
            .await
            .unwrap()
    }

    pub async fn increase_f_asset<T: Account>(
        fpt_staking: &FPTStaking<T>,
        asset_fee_amount: u64,
        asset_address: ContractId,
    ) -> FuelCallResponse<()> {
        // let tx_params = TxParameters::default().set_gas_price(1);

        fpt_staking
            .methods()
            .increase_f_asset(asset_fee_amount, asset_address)
            .call()
            .await
            .unwrap()
    }
}