use fuels::{prelude::abigen, programs::call_response::FuelCallResponse};
abigen!(Contract(
    name = "FPTToken",
    abi = "contracts/fpt-token-contract/out/debug/fpt-token-contract-abi.json"
));

pub mod fpt_token_abi {
    use crate::interfaces::community_issuance::CommunityIssuance;
    use crate::interfaces::vesting::VestingContract;
    use fuels::{prelude::*, types::ContractId};

    use super::*;
    pub async fn initialize<T: Account>(
        instance: &FPTToken<T>,
        vesting_contract: &VestingContract<T>,
        community_issuance_contract: &CommunityIssuance<T>,
    ) -> FuelCallResponse<()> {
        let tx_params = TxParameters::default().with_gas_price(1);

        let res = instance
            .methods()
            .initialize(
                vesting_contract.contract_id(),
                community_issuance_contract.contract_id(),
            )
            .with_contracts(&[vesting_contract, community_issuance_contract])
            .tx_params(tx_params)
            .append_variable_outputs(10)
            .call()
            .await;

        return res.unwrap();
    }

    pub async fn total_supply<T: Account>(instance: &FPTToken<T>) -> FuelCallResponse<Option<u64>> {
        let fpt_token_asset_id = instance
            .contract_id()
            .asset_id(&BASE_ASSET_ID.into())
            .into();

        instance
            .methods()
            .total_supply(fpt_token_asset_id)
            .call()
            .await
            .unwrap()
    }

    pub async fn get_vesting_contract<T: Account>(
        instance: &FPTToken<T>,
    ) -> FuelCallResponse<ContractId> {
        instance
            .methods()
            .get_vesting_contract()
            .call()
            .await
            .unwrap()
    }
}
