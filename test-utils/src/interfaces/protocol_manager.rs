use fuels::prelude::abigen;
use fuels::programs::responses::CallResponse;

abigen!(Contract(
    name = "ProtocolManager",
    abi = "contracts/protocol-manager-contract/out/debug/protocol-manager-contract-abi.json"
));

pub mod protocol_manager_abi {
    use super::*;
    use crate::data_structures;
    use crate::interfaces::active_pool::ActivePool;
    use crate::interfaces::borrow_operations::BorrowOperations;
    use crate::interfaces::coll_surplus_pool::CollSurplusPool;
    use crate::interfaces::default_pool::DefaultPool;
    use crate::interfaces::fpt_staking::FPTStaking;
    use crate::interfaces::sorted_troves::SortedTroves;
    use crate::interfaces::stability_pool::StabilityPool;
    use crate::interfaces::usdf_token::USDFToken;
    use data_structures::AssetContracts;
    use fuels::prelude::{Account, CallParameters, ContractDependency};
    use fuels::types::transaction_builders::VariableOutputPolicy;
    use fuels::types::{Address, AssetId};
    use fuels::{
        prelude::{ContractId, TxPolicies},
        types::Identity,
    };

    pub async fn initialize<T: Account>(
        protocol_manager: &ProtocolManager<T>,
        borrow_operations: ContractId,
        stability_pool: ContractId,
        fpt_staking: ContractId,
        usdf: ContractId,
        coll_surplus_pool: ContractId,
        default_pool: ContractId,
        active_pool: ContractId,
        sorted_troves: ContractId,
        admin: Identity,
    ) -> CallResponse<()> {
        let tx_params = TxPolicies::default().with_tip(1);

        let res = protocol_manager
            .methods()
            .initialize(
                borrow_operations,
                stability_pool,
                fpt_staking,
                usdf,
                coll_surplus_pool,
                default_pool,
                active_pool,
                sorted_troves,
                admin,
            )
            .with_tx_policies(tx_params)
            .call()
            .await
            .unwrap();

        return res;
    }

    pub async fn register_asset<T: Account>(
        protocol_manager: &ProtocolManager<T>,
        asset: AssetId,
        trove_manager: ContractId,
        oracle: ContractId,
        borrow_operations: &BorrowOperations<T>,
        stability_pool: &StabilityPool<T>,
        usdf: &USDFToken<T>,
        fpt_staking: &FPTStaking<T>,
        coll_surplus_pool: &CollSurplusPool<T>,
        default_pool: &DefaultPool<T>,
        active_pool: &ActivePool<T>,
        sorted_troves: &SortedTroves<T>,
    ) -> CallResponse<()> {
        let tx_params = TxPolicies::default().with_tip(1);

        protocol_manager
            .methods()
            .register_asset(asset.into(), trove_manager, oracle)
            .with_tx_policies(tx_params)
            .with_contracts(&[
                borrow_operations,
                stability_pool,
                usdf,
                fpt_staking,
                coll_surplus_pool,
                default_pool,
                active_pool,
                sorted_troves,
            ])
            .call()
            .await
            .unwrap()
    }

    pub async fn redeem_collateral<T: Account>(
        protocol_manager: &ProtocolManager<T>,
        amount: u64,
        max_iterations: u64,
        partial_redemption_hint: u64,
        upper_partial_hint: Option<Identity>,
        lower_partial_hint: Option<Identity>,
        usdf: &USDFToken<T>,
        fpt_staking: &FPTStaking<T>,
        coll_surplus_pool: &CollSurplusPool<T>,
        default_pool: &DefaultPool<T>,
        active_pool: &ActivePool<T>,
        sorted_troves: &SortedTroves<T>,
        aswith_contracts: &Vec<AssetContracts<T>>,
    ) -> CallResponse<()> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_witness_limit(2000000)
            .with_script_gas_limit(2000000);
        let usdf_asset_id = usdf
            .contract_id()
            .asset_id(&AssetId::zeroed().into())
            .into();

        let call_params: CallParameters = CallParameters::default()
            .with_amount(amount)
            .with_asset_id(usdf_asset_id);

        let mut with_contracts: Vec<&dyn ContractDependency> = Vec::new();

        for contracts in aswith_contracts.iter() {
            with_contracts.push(&contracts.trove_manager);
            with_contracts.push(&contracts.oracle);
            with_contracts.push(&contracts.mock_pyth_oracle);
            match &contracts.mock_redstone_oracle {
                Some(redstone) => with_contracts.push(redstone),
                None => (),
            }
        }

        with_contracts.push(fpt_staking);
        with_contracts.push(coll_surplus_pool);
        with_contracts.push(default_pool);
        with_contracts.push(active_pool);
        with_contracts.push(usdf);
        with_contracts.push(sorted_troves);

        protocol_manager
            .methods()
            .redeem_collateral(
                max_iterations,
                partial_redemption_hint,
                upper_partial_hint.unwrap_or(Identity::Address(Address::zeroed())),
                lower_partial_hint.unwrap_or(Identity::Address(Address::zeroed())),
            )
            .with_tx_policies(tx_params)
            .call_params(call_params)
            .unwrap()
            .with_contracts(&with_contracts)
            .with_variable_output_policy(VariableOutputPolicy::Exactly(10))
            .call()
            .await
            .unwrap()
    }

    pub async fn owner<T: Account>(protocol_manager: &ProtocolManager<T>) -> CallResponse<State> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_witness_limit(2000000)
            .with_script_gas_limit(2000000);

        protocol_manager
            .methods()
            .owner()
            .with_tx_policies(tx_params)
            .call()
            .await
            .unwrap()
    }
}
