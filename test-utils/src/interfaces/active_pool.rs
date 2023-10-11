use fuels::prelude::abigen;
use fuels::prelude::BASE_ASSET_ID;
use fuels::programs::call_response::FuelCallResponse;
use fuels::programs::call_utils::TxDependencyExtension;
abigen!(Contract(
    name = "ActivePool",
    abi = "contracts/active-pool-contract/out/debug/active-pool-contract-abi.json"
));

pub mod active_pool_abi {
    use super::*;
    use crate::interfaces::default_pool::DefaultPool;
    use crate::interfaces::token::Token;
    use fuels::prelude::Account;
    use fuels::{
        prelude::{CallParameters, ContractId, Error, TxParameters},
        types::{AssetId, Identity},
    };

    pub async fn initialize<T: Account>(
        active_pool: &ActivePool<T>,
        borrow_operations: Identity,
        stability_pool: Identity,
        default_pool: ContractId,
        protocol_manager: Identity,
    ) -> Result<FuelCallResponse<()>, Error> {
        let tx_params = TxParameters::default().with_gas_price(1);

        let res = active_pool
            .methods()
            .initialize(
                borrow_operations.clone(),
                stability_pool.clone(),
                default_pool,
                protocol_manager,
            )
            .tx_params(tx_params)
            .call()
            .await;

        return res;
    }

    pub async fn get_usdf_debt<T: Account>(
        active_pool: &ActivePool<T>,
        asset_id: AssetId,
    ) -> FuelCallResponse<u64> {
        active_pool
            .methods()
            .get_usdf_debt(asset_id.into())
            .call()
            .await
            .unwrap()
    }

    pub async fn get_asset<T: Account>(
        active_pool: &ActivePool<T>,
        asset_id: AssetId,
    ) -> FuelCallResponse<u64> {
        active_pool
            .methods()
            .get_asset(asset_id.into())
            .call()
            .await
            .unwrap()
    }

    pub async fn increase_usdf_debt<T: Account>(
        active_pool: &ActivePool<T>,
        amount: u64,
        asset_id: AssetId,
    ) -> FuelCallResponse<()> {
        let tx_params = TxParameters::default().with_gas_price(1);

        active_pool
            .methods()
            .increase_usdf_debt(amount, asset_id.into())
            .tx_params(tx_params)
            .call()
            .await
            .unwrap()
    }

    pub async fn decrease_usdf_debt<T: Account>(
        active_pool: &ActivePool<T>,
        amount: u64,
        asset_id: AssetId,
    ) -> FuelCallResponse<()> {
        let tx_params = TxParameters::default().with_gas_price(1);

        active_pool
            .methods()
            .decrease_usdf_debt(amount, asset_id.into())
            .tx_params(tx_params)
            .call()
            .await
            .unwrap()
    }

    pub async fn add_asset<T: Account>(
        active_pool: &ActivePool<T>,
        asset_id: AssetId,
        trove_manager: Identity,
    ) -> FuelCallResponse<()> {
        let tx_params = TxParameters::default().with_gas_price(1);

        active_pool
            .methods()
            .add_asset(asset_id.into(), trove_manager)
            .tx_params(tx_params)
            .call()
            .await
            .unwrap()
    }

    pub async fn recieve<T: Account>(
        active_pool: &ActivePool<T>,
        token: &Token<T>,
        amount: u64,
    ) -> FuelCallResponse<()> {
        let fuel_asset_id = token.contract_id().asset_id(&BASE_ASSET_ID.into()).into();

        let call_params: CallParameters = CallParameters::default()
            .with_amount(amount)
            .with_asset_id(fuel_asset_id);

        active_pool
            .methods()
            .recieve()
            .call_params(call_params)
            .unwrap()
            .with_contracts(&[token])
            .append_variable_outputs(2)
            .call()
            .await
            .unwrap()
    }

    pub async fn send_asset<T: Account>(
        active_pool: &ActivePool<T>,
        recipient: Identity,
        amount: u64,
        asset_id: AssetId,
    ) -> FuelCallResponse<()> {
        active_pool
            .methods()
            .send_asset(recipient, amount, asset_id.into())
            .append_variable_outputs(1)
            .call()
            .await
            .unwrap()
    }

    pub async fn send_asset_to_default_pool<T: Account>(
        active_pool: &ActivePool<T>,
        default_pool: &DefaultPool<T>,
        asset: &Token<T>,
        amount: u64,
    ) -> Result<FuelCallResponse<()>, Error> {
        active_pool
            .methods()
            .send_asset_to_default_pool(
                amount,
                asset.contract_id().asset_id(&BASE_ASSET_ID.into()).into(),
            )
            .with_contracts(&[default_pool, asset])
            .append_variable_outputs(1)
            .call()
            .await
    }
}
