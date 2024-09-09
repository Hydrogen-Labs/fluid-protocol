use fuels::{prelude::abigen, programs::responses::CallResponse, types::Identity};

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
        debugging: bool,
    ) -> CallResponse<()> {
        let tx_params = TxPolicies::default().with_tip(1);

        let res = instance
            .methods()
            .initialize(
                vesting_contract.contract_id(),
                community_issuance_contract.contract_id(),
                debugging,
            )
            .with_contracts(&[vesting_contract, community_issuance_contract])
            .with_tx_policies(tx_params)
            .with_variable_output_policy(VariableOutputPolicy::Exactly(10))
            .call()
            .await;

        return res.unwrap();
    }

    pub async fn total_supply<T: Account>(instance: &FPTToken<T>) -> CallResponse<Option<u64>> {
        let fpt_token_asset_id = instance
            .contract_id()
            .asset_id(&AssetId::zeroed().into())
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
    ) -> CallResponse<ContractId> {
        instance
            .methods()
            .get_vesting_contract()
            .call()
            .await
            .unwrap()
    }

    pub async fn mint_to_id<T: Account>(
        instance: &FPTToken<T>,
        amount: u64,
        admin: Identity,
    ) -> CallResponse<()> {
        instance
            .methods()
            .mint_to_id(amount, admin)
            .with_variable_output_policy(VariableOutputPolicy::Exactly(1))
            .call()
            .await
            .unwrap()
    }
}
