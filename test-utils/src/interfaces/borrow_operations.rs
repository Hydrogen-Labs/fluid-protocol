use fuels::prelude::abigen;
use fuels::programs::responses::CallResponse;

abigen!(Contract(
    name = "BorrowOperations",
    abi = "contracts/borrow-operations-contract/out/debug/borrow-operations-contract-abi.json"
));

pub mod borrow_operations_abi {
    use super::*;
    use crate::data_structures::ContractInstance;
    use crate::interfaces::active_pool::ActivePool;
    use crate::interfaces::coll_surplus_pool::CollSurplusPool;
    use crate::interfaces::default_pool::DefaultPool;
    use crate::interfaces::fpt_staking::FPTStaking;
    use crate::interfaces::oracle::Oracle;
    use crate::interfaces::pyth_oracle::PythCore;
    use crate::interfaces::redstone_oracle::RedstoneCore;
    use crate::interfaces::sorted_troves::SortedTroves;
    use crate::interfaces::token::Token;
    use crate::interfaces::trove_manager::TroveManagerContract;
    use crate::interfaces::usdf_token::USDFToken;
    use fuels::prelude::Account;
    use fuels::prelude::{CallParameters, ContractId, Error, TxPolicies};
    use fuels::types::transaction_builders::VariableOutputPolicy;
    use fuels::types::{AssetId, Identity};

    pub async fn initialize<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        usdf_contract: ContractId,
        fpt_staking_contract: ContractId,
        protocol_manager_contract: ContractId,
        coll_surplus_pool_contract: ContractId,
        active_pool_contract: ContractId,
        sorted_troves_contract: ContractId,
    ) -> CallResponse<()> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_script_gas_limit(2000000);

        borrow_operations
            .contract
            .methods()
            .initialize(
                usdf_contract,
                fpt_staking_contract,
                protocol_manager_contract,
                coll_surplus_pool_contract,
                active_pool_contract,
                sorted_troves_contract,
            )
            .with_tx_policies(tx_params)
            .with_contract_ids(&[borrow_operations.implementation_id.into()])
            .call()
            .await
            .unwrap()
    }

    pub async fn open_trove<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        oracle: &ContractInstance<Oracle<T>>,
        mock_pyth: &PythCore<T>,
        _mock_redstone: &RedstoneCore<T>,
        asset_token: &Token<T>,
        usdf_token: &ContractInstance<USDFToken<T>>,
        fpt_staking: &ContractInstance<FPTStaking<T>>,
        sorted_troves: &ContractInstance<SortedTroves<T>>,
        trove_manager: &ContractInstance<TroveManagerContract<T>>,
        active_pool: &ContractInstance<ActivePool<T>>,
        collateral_amount_deposit: u64,
        usdf_amount_withdrawn: u64,
        upper_hint: Identity,
        lower_hint: Identity,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default().with_tip(1);

        let asset_id = asset_token
            .contract_id()
            .asset_id(&AssetId::zeroed().into())
            .into();

        let call_params: CallParameters = CallParameters::default()
            .with_amount(collateral_amount_deposit)
            .with_asset_id(asset_id);

        return borrow_operations
            .contract
            .methods()
            .open_trove(usdf_amount_withdrawn, upper_hint, lower_hint)
            .call_params(call_params)
            .unwrap()
            .with_contracts(&[
                &oracle.contract,
                mock_pyth,
                //mock_redstone,
                &active_pool.contract,
                &usdf_token.contract,
                &sorted_troves.contract,
                &trove_manager.contract,
                &fpt_staking.contract,
            ])
            .with_contract_ids(&[
                borrow_operations.contract.contract_id().into(),
                borrow_operations.implementation_id.into(),
                sorted_troves.implementation_id.into(),
                sorted_troves.contract.contract_id().into(),
                fpt_staking.contract.contract_id().into(),
                fpt_staking.implementation_id.into(),
                oracle.contract.contract_id().into(),
                oracle.implementation_id.into(),
                mock_pyth.contract_id().into(),
                active_pool.contract.contract_id().into(),
                active_pool.implementation_id.into(),
                usdf_token.contract.contract_id().into(),
                usdf_token.implementation_id.into(),
                trove_manager.contract.contract_id().into(),
                trove_manager.implementation_id.into(),
            ])
            .with_variable_output_policy(VariableOutputPolicy::Exactly(3))
            .with_tx_policies(tx_params)
            .call()
            .await;
    }

    pub async fn add_coll<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        oracle: &ContractInstance<Oracle<T>>,
        pyth: &PythCore<T>,
        redstone: &RedstoneCore<T>,
        mock_token: &Token<T>,
        usdf_token: &ContractInstance<USDFToken<T>>,
        sorted_troves: &ContractInstance<SortedTroves<T>>,
        trove_manager: &ContractInstance<TroveManagerContract<T>>,
        active_pool: &ContractInstance<ActivePool<T>>,
        amount: u64,
        lower_hint: Identity,
        upper_hint: Identity,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_script_gas_limit(2000000);

        let mock_asset_id: AssetId = mock_token
            .contract_id()
            .asset_id(&AssetId::zeroed().into())
            .into();

        let call_params: CallParameters = CallParameters::default()
            .with_amount(amount)
            .with_asset_id(mock_asset_id);

        borrow_operations
            .contract
            .methods()
            .add_coll(lower_hint, upper_hint)
            .call_params(call_params)
            .unwrap()
            .with_contracts(&[
                &oracle.contract,
                pyth,
                redstone,
                mock_token,
                &sorted_troves.contract,
                &trove_manager.contract,
                &active_pool.contract,
                &usdf_token.contract,
            ])
            .with_contract_ids(&[
                borrow_operations.contract.contract_id().into(),
                borrow_operations.implementation_id.into(),
                sorted_troves.implementation_id.into(),
                sorted_troves.contract.contract_id().into(),
                trove_manager.contract.contract_id().into(),
                trove_manager.implementation_id.into(),
                oracle.contract.contract_id().into(),
                oracle.implementation_id.into(),
                pyth.contract_id().into(),
                redstone.contract_id().into(),
                mock_token.contract_id().into(),
                usdf_token.contract.contract_id().into(),
                usdf_token.implementation_id.into(),
                active_pool.contract.contract_id().into(),
                active_pool.implementation_id.into(),
            ])
            .with_variable_output_policy(VariableOutputPolicy::Exactly(1))
            .with_tx_policies(tx_params)
            .call()
            .await
    }

    pub async fn withdraw_coll<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        oracle: &ContractInstance<Oracle<T>>,
        pyth: &PythCore<T>,
        redstone: &RedstoneCore<T>,
        mock_token: &Token<T>,
        sorted_troves: &ContractInstance<SortedTroves<T>>,
        trove_manager: &ContractInstance<TroveManagerContract<T>>,
        active_pool: &ContractInstance<ActivePool<T>>,
        amount: u64,
        lower_hint: Identity,
        upper_hint: Identity,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_script_gas_limit(2000000);

        let mock_asset_id: AssetId = mock_token
            .contract_id()
            .asset_id(&AssetId::zeroed().into())
            .into();

        borrow_operations
            .contract
            .methods()
            .withdraw_coll(amount, lower_hint, upper_hint, mock_asset_id.into())
            .with_contracts(&[
                &oracle.contract,
                pyth,
                redstone,
                mock_token,
                &sorted_troves.contract,
                &trove_manager.contract,
                &active_pool.contract,
            ])
            .with_contract_ids(&[
                borrow_operations.contract.contract_id().into(),
                borrow_operations.implementation_id.into(),
                sorted_troves.implementation_id.into(),
                sorted_troves.contract.contract_id().into(),
                trove_manager.contract.contract_id().into(),
                trove_manager.implementation_id.into(),
                oracle.contract.contract_id().into(),
                oracle.implementation_id.into(),
                pyth.contract_id().into(),
                redstone.contract_id().into(),
                mock_token.contract_id().into(),
                active_pool.contract.contract_id().into(),
                active_pool.implementation_id.into(),
            ])
            .with_variable_output_policy(VariableOutputPolicy::Exactly(1))
            .with_tx_policies(tx_params)
            .call()
            .await
    }

    pub async fn withdraw_usdf<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        oracle: &ContractInstance<Oracle<T>>,
        pyth: &PythCore<T>,
        redstone: &RedstoneCore<T>,
        mock_token: &Token<T>,
        usdf_token: &ContractInstance<USDFToken<T>>,
        fpt_staking: &ContractInstance<FPTStaking<T>>,
        sorted_troves: &ContractInstance<SortedTroves<T>>,
        trove_manager: &ContractInstance<TroveManagerContract<T>>,
        active_pool: &ContractInstance<ActivePool<T>>,
        amount: u64,
        lower_hint: Identity,
        upper_hint: Identity,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_script_gas_limit(2000000);

        let mock_asset_id: AssetId = mock_token
            .contract_id()
            .asset_id(&AssetId::zeroed().into())
            .into();

        borrow_operations
            .contract
            .methods()
            .withdraw_usdf(amount, lower_hint, upper_hint, mock_asset_id.into())
            .with_contracts(&[
                &oracle.contract,
                pyth,
                redstone,
                mock_token,
                &sorted_troves.contract,
                &trove_manager.contract,
                &active_pool.contract,
                &usdf_token.contract,
                &fpt_staking.contract,
            ])
            .with_contract_ids(&[
                borrow_operations.contract.contract_id().into(),
                borrow_operations.implementation_id.into(),
                sorted_troves.implementation_id.into(),
                sorted_troves.contract.contract_id().into(),
                trove_manager.contract.contract_id().into(),
                trove_manager.implementation_id.into(),
                oracle.contract.contract_id().into(),
                oracle.implementation_id.into(),
                pyth.contract_id().into(),
                redstone.contract_id().into(),
                mock_token.contract_id().into(),
                usdf_token.contract.contract_id().into(),
                usdf_token.implementation_id.into(),
                active_pool.contract.contract_id().into(),
                active_pool.implementation_id.into(),
                fpt_staking.contract.contract_id().into(),
                fpt_staking.implementation_id.into(),
            ])
            .with_variable_output_policy(VariableOutputPolicy::Exactly(1))
            .with_tx_policies(tx_params)
            .call()
            .await
    }

    pub async fn repay_usdf<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        oracle: &ContractInstance<Oracle<T>>,
        pyth: &PythCore<T>,
        redstone: &RedstoneCore<T>,
        mock_token: &Token<T>,
        usdf_token: &ContractInstance<USDFToken<T>>,
        sorted_troves: &ContractInstance<SortedTroves<T>>,
        trove_manager: &ContractInstance<TroveManagerContract<T>>,
        active_pool: &ContractInstance<ActivePool<T>>,
        default_pool: &ContractInstance<DefaultPool<T>>,
        amount: u64,
        lower_hint: Identity,
        upper_hint: Identity,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_script_gas_limit(2000000);
        let usdf_asset_id = usdf_token
            .contract
            .contract_id()
            .asset_id(&AssetId::zeroed().into())
            .into();

        let call_params: CallParameters = CallParameters::default()
            .with_amount(amount)
            .with_asset_id(usdf_asset_id);

        let mock_asset_id: AssetId = mock_token
            .contract_id()
            .asset_id(&AssetId::zeroed().into())
            .into();

        borrow_operations
            .contract
            .methods()
            .repay_usdf(lower_hint, upper_hint, mock_asset_id.into())
            .with_contracts(&[
                &oracle.contract,
                pyth,
                redstone,
                mock_token,
                &sorted_troves.contract,
                &trove_manager.contract,
                &active_pool.contract,
                &usdf_token.contract,
                &default_pool.contract,
            ])
            .with_contract_ids(&[
                borrow_operations.contract.contract_id().into(),
                borrow_operations.implementation_id.into(),
                sorted_troves.implementation_id.into(),
                sorted_troves.contract.contract_id().into(),
                trove_manager.contract.contract_id().into(),
                trove_manager.implementation_id.into(),
                oracle.contract.contract_id().into(),
                oracle.implementation_id.into(),
                pyth.contract_id().into(),
                redstone.contract_id().into(),
                mock_token.contract_id().into(),
                usdf_token.contract.contract_id().into(),
                usdf_token.implementation_id.into(),
                active_pool.contract.contract_id().into(),
                active_pool.implementation_id.into(),
                default_pool.contract.contract_id().into(),
                default_pool.implementation_id.into(),
            ])
            .with_variable_output_policy(VariableOutputPolicy::Exactly(1))
            .with_tx_policies(tx_params)
            .call_params(call_params)
            .unwrap()
            .call()
            .await
    }

    pub async fn close_trove<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        oracle: &ContractInstance<Oracle<T>>,
        pyth: &PythCore<T>,
        redstone: &RedstoneCore<T>,
        mock_token: &Token<T>,
        usdf_token: &ContractInstance<USDFToken<T>>,
        fpt_staking: &ContractInstance<FPTStaking<T>>,
        sorted_troves: &ContractInstance<SortedTroves<T>>,
        trove_manager: &ContractInstance<TroveManagerContract<T>>,
        active_pool: &ContractInstance<ActivePool<T>>,
        amount: u64,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_script_gas_limit(2000000);
        let usdf_asset_id: AssetId = usdf_token
            .contract
            .contract_id()
            .asset_id(&AssetId::zeroed().into())
            .into();

        println!("usdf_asset_id: {:?}", usdf_asset_id);

        let call_params: CallParameters = CallParameters::default()
            .with_amount(amount)
            .with_asset_id(usdf_asset_id);

        let mock_asset_id: AssetId = mock_token
            .contract_id()
            .asset_id(&AssetId::zeroed().into())
            .into();

        borrow_operations
            .contract
            .methods()
            .close_trove(mock_asset_id.into())
            .with_contracts(&[
                &oracle.contract,
                pyth,
                redstone,
                mock_token,
                &sorted_troves.contract,
                &trove_manager.contract,
                &active_pool.contract,
                &usdf_token.contract,
                &fpt_staking.contract,
            ])
            .with_contract_ids(&[
                borrow_operations.contract.contract_id().into(),
                borrow_operations.implementation_id.into(),
                sorted_troves.implementation_id.into(),
                sorted_troves.contract.contract_id().into(),
                trove_manager.contract.contract_id().into(),
                trove_manager.implementation_id.into(),
                oracle.contract.contract_id().into(),
                oracle.implementation_id.into(),
                pyth.contract_id().into(),
                redstone.contract_id().into(),
                mock_token.contract_id().into(),
                usdf_token.contract.contract_id().into(),
                usdf_token.implementation_id.into(),
                active_pool.contract.contract_id().into(),
                active_pool.implementation_id.into(),
                fpt_staking.contract.contract_id().into(),
                fpt_staking.implementation_id.into(),
            ])
            .with_variable_output_policy(VariableOutputPolicy::Exactly(3))
            .with_tx_policies(tx_params)
            .call_params(call_params)
            .unwrap()
            .call()
            .await
    }

    pub async fn add_asset<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        oracle: ContractId,
        trove_manager: ContractId,
        asset: AssetId,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_script_gas_limit(2000000);

        return borrow_operations
            .contract
            .methods()
            .add_asset(asset.into(), trove_manager, oracle)
            .with_tx_policies(tx_params)
            .call()
            .await;
    }

    pub async fn set_pause_status<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        is_paused: bool,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_script_gas_limit(2000000);

        borrow_operations
            .contract
            .methods()
            .set_pause_status(is_paused)
            .with_contract_ids(&[borrow_operations.implementation_id.into()])
            .with_tx_policies(tx_params)
            .call()
            .await
    }

    pub async fn get_pauser<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
    ) -> Result<CallResponse<Identity>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_script_gas_limit(2000000);

        borrow_operations
            .contract
            .methods()
            .get_pauser()
            .with_contract_ids(&[borrow_operations.implementation_id.into()])
            .with_tx_policies(tx_params)
            .call()
            .await
    }

    pub async fn get_is_paused<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
    ) -> Result<CallResponse<bool>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_script_gas_limit(2000000);

        borrow_operations
            .contract
            .methods()
            .get_is_paused()
            .with_contract_ids(&[borrow_operations.implementation_id.into()])
            .with_tx_policies(tx_params)
            .call()
            .await
    }

    pub async fn get_asset_pause_status<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        asset: AssetId,
    ) -> Result<CallResponse<bool>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_script_gas_limit(2000000);

        borrow_operations
            .contract
            .methods()
            .get_asset_pause_status(asset.into())
            .with_contract_ids(&[borrow_operations.implementation_id.into()])
            .with_tx_policies(tx_params)
            .call()
            .await
    }

    pub async fn set_asset_pause_status<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        asset: AssetId,
        is_paused: bool,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default().with_tip(1);

        borrow_operations
            .contract
            .methods()
            .set_asset_pause_status(asset.into(), is_paused)
            .with_contract_ids(&[borrow_operations.implementation_id.into()])
            .with_tx_policies(tx_params)
            .call()
            .await
    }

    pub async fn claim_coll<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        active_pool: &ContractInstance<ActivePool<T>>,
        coll_surplus_pool: &ContractInstance<CollSurplusPool<T>>,
        asset: AssetId,
    ) -> CallResponse<()> {
        borrow_operations
            .contract
            .methods()
            .claim_collateral(asset.into())
            .with_contracts(&[&active_pool.contract, &coll_surplus_pool.contract])
            .with_variable_output_policy(VariableOutputPolicy::Exactly(1))
            .with_contract_ids(&[
                borrow_operations.contract.contract_id().into(),
                borrow_operations.implementation_id.into(),
                active_pool.contract.contract_id().into(),
                active_pool.implementation_id.into(),
                coll_surplus_pool.contract.contract_id().into(),
                coll_surplus_pool.implementation_id.into(),
            ])
            .call()
            .await
            .unwrap()
    }

    // Add these new functions to the module
    pub async fn set_pauser<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        pauser: Identity,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_script_gas_limit(2000000);

        borrow_operations
            .contract
            .methods()
            .set_pauser(pauser)
            .with_contract_ids(&[borrow_operations.implementation_id.into()])
            .with_tx_policies(tx_params)
            .call()
            .await
    }

    pub async fn transfer_owner<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        new_owner: Identity,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_script_gas_limit(2000000);

        borrow_operations
            .contract
            .methods()
            .transfer_owner(new_owner)
            .with_contract_ids(&[borrow_operations.implementation_id.into()])
            .with_tx_policies(tx_params)
            .call()
            .await
    }

    pub async fn renounce_owner<T: Account>(
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
    ) -> Result<CallResponse<()>, Error> {
        let tx_params = TxPolicies::default()
            .with_tip(1)
            .with_script_gas_limit(2000000);

        borrow_operations
            .contract
            .methods()
            .renounce_owner()
            .with_contract_ids(&[borrow_operations.implementation_id.into()])
            .with_tx_policies(tx_params)
            .call()
            .await
    }
}

pub mod borrow_operations_utils {
    use fuels::prelude::{Account, WalletUnlocked};
    use fuels::types::{Address, Identity};

    use super::*;
    use crate::data_structures::ContractInstance;
    use crate::interfaces::active_pool::ActivePool;
    use crate::interfaces::fpt_staking::FPTStaking;
    use crate::interfaces::sorted_troves::SortedTroves;
    use crate::interfaces::usdf_token::USDFToken;
    use crate::{data_structures::AssetContracts, interfaces::token::token_abi};

    pub async fn mint_token_and_open_trove<T: Account>(
        wallet: WalletUnlocked,
        asset_contracts: &AssetContracts<WalletUnlocked>,
        borrow_operations: &ContractInstance<BorrowOperations<T>>,
        usdf: &ContractInstance<USDFToken<WalletUnlocked>>,
        fpt_staking: &ContractInstance<FPTStaking<WalletUnlocked>>,
        active_pool: &ContractInstance<ActivePool<WalletUnlocked>>,
        sorted_troves: &ContractInstance<SortedTroves<WalletUnlocked>>,
        amount: u64,
        usdf_amount: u64,
    ) {
        token_abi::mint_to_id(
            &asset_contracts.asset,
            amount,
            Identity::Address(wallet.address().into()),
        )
        .await;

        let borrow_operations_healthy_wallet1 = ContractInstance::new(
            BorrowOperations::new(
                borrow_operations.contract.contract_id().clone(),
                wallet.clone(),
            ),
            borrow_operations.implementation_id.into(),
        );

        borrow_operations_abi::open_trove(
            &borrow_operations_healthy_wallet1,
            &asset_contracts.oracle,
            &asset_contracts.mock_pyth_oracle,
            &asset_contracts.mock_redstone_oracle,
            &asset_contracts.asset,
            &usdf,
            fpt_staking,
            &sorted_troves,
            &asset_contracts.trove_manager,
            &active_pool,
            amount,
            usdf_amount,
            Identity::Address(Address::zeroed()),
            Identity::Address(Address::zeroed()),
        )
        .await
        .unwrap();
    }
}
