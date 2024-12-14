use fuels::prelude::abigen;
use fuels::programs::responses::CallResponse;

abigen!(Contract(
    name = "ProtocolManager",
    abi = "contracts/protocol-manager-contract/out/debug/protocol-manager-contract-abi.json"
));

pub mod protocol_manager_abi {
    use super::*;
    use crate::data_structures::{self, ContractInstance};
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
    use fuels::types::bech32::Bech32ContractId;
    use fuels::types::errors::Error;
    use fuels::types::transaction_builders::VariableOutputPolicy;
    use fuels::types::{Address, AssetId};
    use fuels::{
        prelude::{ContractId, TxPolicies},
        types::Identity,
    };

    pub async fn initialize<T: Account>(
        protocol_manager: &ContractInstance<ProtocolManager<T>>,
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
            .contract
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
            .with_contract_ids(&[
                protocol_manager.contract.contract_id().into(),
                protocol_manager.implementation_id.into(),
            ])
            .call()
            .await
            .unwrap();

        return res;
    }

    pub async fn register_asset<T: Account>(
        protocol_manager: &ContractInstance<ProtocolManager<T>>,
        asset: AssetId,
        trove_manager: ContractId,
        oracle: ContractId,
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        stability_pool: &ContractInstance<StabilityPool<T>>,
        usdf: &ContractInstance<USDFToken<T>>,
        fpt_staking: &ContractInstance<FPTStaking<T>>,
        coll_surplus_pool: &ContractInstance<CollSurplusPool<T>>,
        default_pool: &ContractInstance<DefaultPool<T>>,
        active_pool: &ContractInstance<ActivePool<T>>,
        sorted_troves: &ContractInstance<SortedTroves<T>>,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default().with_tip(1);
        protocol_manager
            .contract
            .methods()
            .register_asset(asset.into(), trove_manager, oracle)
            .with_tx_policies(tx_params)
            .with_contracts(&[
                &borrow_operations.contract,
                &stability_pool.contract,
                &usdf.contract,
                &fpt_staking.contract,
                &coll_surplus_pool.contract,
                &default_pool.contract,
                &active_pool.contract,
                &sorted_troves.contract,
            ])
            .with_contract_ids(&[
                protocol_manager.contract.contract_id().into(),
                protocol_manager.implementation_id.into(),
                borrow_operations.contract.contract_id().into(),
                borrow_operations.implementation_id.into(),
                sorted_troves.implementation_id.into(),
                sorted_troves.contract.contract_id().into(),
                coll_surplus_pool.contract.contract_id().into(),
                coll_surplus_pool.implementation_id.into(),
                default_pool.contract.contract_id().into(),
                default_pool.implementation_id.into(),
                active_pool.contract.contract_id().into(),
                active_pool.implementation_id.into(),
                fpt_staking.contract.contract_id().into(),
                fpt_staking.implementation_id.into(),
                usdf.contract.contract_id().into(),
                usdf.implementation_id.into(),
                stability_pool.contract.contract_id().into(),
                stability_pool.implementation_id.into(),
            ])
            .call()
            .await
    }

    pub async fn redeem_collateral<T: Account>(
        protocol_manager: &ContractInstance<ProtocolManager<T>>,
        amount: u64,
        max_iterations: u64,
        partial_redemption_hint: u64,
        upper_partial_hint: Option<Identity>,
        lower_partial_hint: Option<Identity>,
        usdf: &ContractInstance<USDFToken<T>>,
        fpt_staking: &ContractInstance<FPTStaking<T>>,
        coll_surplus_pool: &ContractInstance<CollSurplusPool<T>>,
        default_pool: &ContractInstance<DefaultPool<T>>,
        active_pool: &ContractInstance<ActivePool<T>>,
        sorted_troves: &ContractInstance<SortedTroves<T>>,
        aswith_contracts: &Vec<AssetContracts<T>>,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_witness_limit(2000000)
            .with_script_gas_limit(2000000);
        let usdf_asset_id = usdf
            .contract
            .contract_id()
            .asset_id(&AssetId::zeroed().into())
            .into();

        let call_params: CallParameters = CallParameters::default()
            .with_amount(amount)
            .with_asset_id(usdf_asset_id);

        let mut with_contracts: Vec<&dyn ContractDependency> = Vec::new();

        for contracts in aswith_contracts.iter() {
            with_contracts.push(&contracts.trove_manager.contract);
            with_contracts.push(&contracts.oracle.contract);
            with_contracts.push(&contracts.mock_pyth_oracle);
            with_contracts.push(&contracts.mock_redstone_oracle);
        }

        with_contracts.push(&fpt_staking.contract);
        with_contracts.push(&coll_surplus_pool.contract);
        with_contracts.push(&default_pool.contract);
        with_contracts.push(&active_pool.contract);
        with_contracts.push(&usdf.contract);
        with_contracts.push(&sorted_troves.contract);

        let mut with_contract_ids: Vec<Bech32ContractId> = Vec::new();
        with_contract_ids.push(sorted_troves.contract.contract_id().into());
        with_contract_ids.push(sorted_troves.implementation_id.into());
        with_contract_ids.push(fpt_staking.contract.contract_id().into());
        with_contract_ids.push(fpt_staking.implementation_id.into());
        with_contract_ids.push(coll_surplus_pool.contract.contract_id().into());
        with_contract_ids.push(coll_surplus_pool.implementation_id.into());
        with_contract_ids.push(default_pool.contract.contract_id().into());
        with_contract_ids.push(default_pool.implementation_id.into());
        with_contract_ids.push(active_pool.contract.contract_id().into());
        with_contract_ids.push(active_pool.implementation_id.into());
        with_contract_ids.push(usdf.contract.contract_id().into());
        with_contract_ids.push(usdf.implementation_id.into());
        with_contract_ids.push(sorted_troves.contract.contract_id().into());
        with_contract_ids.push(sorted_troves.implementation_id.into());
        with_contract_ids.push(protocol_manager.contract.contract_id().into());
        with_contract_ids.push(protocol_manager.implementation_id.into());

        for contracts in aswith_contracts.iter() {
            with_contract_ids.push(contracts.trove_manager.contract.contract_id().into());
            with_contract_ids.push(contracts.trove_manager.implementation_id.into());
            with_contract_ids.push(contracts.oracle.contract.contract_id().into());
            with_contract_ids.push(contracts.oracle.implementation_id.into());
            with_contract_ids.push(contracts.mock_pyth_oracle.contract_id().into());
            with_contract_ids.push(contracts.mock_redstone_oracle.contract_id().into());
        }

        protocol_manager
            .contract
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
            .with_contract_ids(&with_contract_ids.to_vec())
            .with_variable_output_policy(VariableOutputPolicy::Exactly(10))
            .call()
            .await
    }

    pub async fn owner<T: Account>(
        protocol_manager: &ContractInstance<ProtocolManager<T>>,
    ) -> CallResponse<State> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_witness_limit(2000000)
            .with_script_gas_limit(2000000);

        protocol_manager
            .contract
            .methods()
            .owner()
            .with_contract_ids(&[
                protocol_manager.contract.contract_id().into(),
                protocol_manager.implementation_id.into(),
            ])
            .with_tx_policies(tx_params)
            .call()
            .await
            .unwrap()
    }

    pub async fn renounce_admin<T: Account>(
        protocol_manager: &ContractInstance<ProtocolManager<T>>,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_witness_limit(2000000)
            .with_script_gas_limit(2000000);

        protocol_manager
            .contract
            .methods()
            .renounce_admin()
            .with_contract_ids(&[
                protocol_manager.contract.contract_id().into(),
                protocol_manager.implementation_id.into(),
            ])
            .with_tx_policies(tx_params)
            .call()
            .await
    }

    pub async fn transfer_owner<T: Account>(
        protocol_manager: &ContractInstance<ProtocolManager<T>>,
        new_owner: Identity,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default().with_tip(1);

        protocol_manager
            .contract
            .methods()
            .transfer_owner(new_owner)
            .with_contract_ids(&[
                protocol_manager.contract.contract_id().into(),
                protocol_manager.implementation_id.into(),
            ])
            .with_tx_policies(tx_params)
            .call()
            .await
    }
}
