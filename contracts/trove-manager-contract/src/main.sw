contract;
// The TroveManager contract is responsible for managing the troves in the system.
// It handles the creation, modification, and deletion of troves, as well as the distribution of rewards to trove owners.
// It also interfaces with other core contracts like StabilityPool, ActivePool, and DefaultPool.
mod data_structures;
mod utils;
mod events;
use ::utils::{add_liquidation_vals_to_totals, get_offset_and_redistribution_vals};
use ::data_structures::{
    EntireTroveDebtAndColl,
    LiquidationTotals,
    LiquidationValues,
    LocalVariablesLiquidationSequence,
    LocalVariablesOuterLiquidationFunction,
    RedemptionTotals,
    Trove,
};
use ::events::{RedemptionEvent, TroveFullLiquidationEvent, TrovePartialLiquidationEvent,};
use standards::src3::SRC3;
use libraries::trove_manager_interface::TroveManager;
use libraries::usdf_token_interface::USDFToken;
use libraries::sorted_troves_interface::SortedTroves;
use libraries::stability_pool_interface::StabilityPool;
use libraries::default_pool_interface::DefaultPool;
use libraries::active_pool_interface::ActivePool;
use libraries::coll_surplus_pool_interface::CollSurplusPool;
use libraries::oracle_interface::Oracle;
use libraries::trove_manager_interface::data_structures::{
    RewardSnapshot,
    SingleRedemptionValues,
    Status,
};
use libraries::fluid_math::*;
use std::{
    asset::transfer,
    block::{
        height,
        timestamp,
    },
    call_frames::{
        msg_asset_id,
    },
    context::{
        msg_amount,
    },
    hash::Hash,
    logging::log,
    storage::storage_vec::*,
    u128::U128,
};
configurable {
    /// Initializer identity
    INITIALIZER: Identity = Identity::Address(Address::zero()),
}
storage {
    protocol_manager_contract: ContractId = ContractId::zero(),
    sorted_troves_contract: ContractId = ContractId::zero(),
    borrow_operations_contract: ContractId = ContractId::zero(),
    stability_pool_contract: ContractId = ContractId::zero(),
    oracle_contract: ContractId = ContractId::zero(),
    active_pool_contract: ContractId = ContractId::zero(),
    default_pool_contract: ContractId = ContractId::zero(),
    coll_surplus_pool_contract: ContractId = ContractId::zero(),
    usdf_contract: ContractId = ContractId::zero(),
    asset_contract: AssetId = AssetId::zero(),
    total_stakes: u64 = 0,
    total_stakes_snapshot: u64 = 0,
    total_collateral_snapshot: u64 = 0,
    l_asset: u64 = 0,
    l_usdf: u64 = 0,
    last_asset_error_redistribution: u64 = 0,
    last_usdf_error_redistribution: u64 = 0,
    troves: StorageMap<Identity, Trove> = StorageMap::<Identity, Trove> {},
    trove_owners: StorageVec<Identity> = StorageVec {},
    reward_snapshots: StorageMap<Identity, RewardSnapshot> = StorageMap::<Identity, RewardSnapshot> {},
    is_initialized: bool = false,
    lock_internal_close_trove: bool = false,
    lock_internal_batch_liquidate_troves: bool = false,
    lock_internal_redeem_collateral_from_trove: bool = false,
}
impl TroveManager for Contract {
    #[storage(read, write)]
    fn initialize(
        borrow_operations: ContractId,
        sorted_troves: ContractId,
        oracle: ContractId,
        stability_pool: ContractId,
        default_pool: ContractId,
        active_pool: ContractId,
        coll_surplus_pool: ContractId,
        usdf_contract: ContractId,
        asset_contract: AssetId,
        protocol_manager: ContractId,
    ) {
        require(
            msg_sender()
                .unwrap() == INITIALIZER,
            "TroveManager: Caller is not initializer",
        );
        require(
            storage
                .is_initialized
                .read() == false,
            "TroveManager: Contract is already initialized",
        );
        storage.sorted_troves_contract.write(sorted_troves);
        storage.borrow_operations_contract.write(borrow_operations);
        storage.stability_pool_contract.write(stability_pool);
        storage.oracle_contract.write(oracle);
        storage.default_pool_contract.write(default_pool);
        storage.active_pool_contract.write(active_pool);
        storage.coll_surplus_pool_contract.write(coll_surplus_pool);
        storage.usdf_contract.write(usdf_contract);
        storage.asset_contract.write(asset_contract);
        storage.protocol_manager_contract.write(protocol_manager);
        storage.is_initialized.write(true);
    }
    #[storage(read)]
    fn get_nominal_icr(id: Identity) -> u64 {
        internal_get_nominal_icr(id)
    }
    #[storage(read, write)]
    fn apply_pending_rewards(id: Identity) {
        require_caller_is_borrow_operations_contract_or_protocol_manager();
        internal_apply_pending_rewards(id);
    }
    #[storage(read)]
    fn has_pending_rewards(id: Identity) -> bool {
        internal_has_pending_rewards(id)
    }
    #[storage(read)]
    fn get_trove_owners_count() -> u64 {
        return storage.trove_owners.len();
    }
    #[storage(read)]
    fn get_trove_owner_by_index(index: u64) -> Identity {
        return storage.trove_owners.get(index).unwrap().read();
    }
    #[storage(read)]
    fn get_trove_rewards_snapshot(id: Identity) -> RewardSnapshot {
        return storage.reward_snapshots.get(id).read();
    }
    #[storage(read, write)]
    fn redeem_collateral_from_trove(
        borrower: Identity,
        max_usdf_amount: u64,
        price: u64,
        partial_redemption_hint: u64,
        upper_partial_hint: Identity,
        lower_partial_hint: Identity,
    ) -> SingleRedemptionValues {
        require_caller_is_protocol_manager_contract();
        internal_redeem_collateral_from_trove(
            borrower,
            max_usdf_amount,
            price,
            partial_redemption_hint,
            upper_partial_hint,
            lower_partial_hint,
        )
    }
    #[storage(read)]
    fn get_current_icr(id: Identity, price: u64) -> u64 {
        internal_get_current_icr(id, price)
    }
    #[storage(read)]
    fn get_entire_system_debt() -> u64 {
        internal_get_entire_system_debt()
    }
    #[storage(read)]
    fn get_entire_debt_and_coll(id: Identity) -> (u64, u64, u64, u64) {
        let res = internal_get_entire_debt_and_coll(id);
        return (
            res.entire_trove_debt,
            res.entire_trove_coll,
            res.pending_debt_rewards,
            res.pending_coll_rewards,
        )
    }
    #[storage(read)]
    fn get_trove_stake(id: Identity) -> u64 {
        internal_get_trove_stake(id)
    }
    #[storage(read, write)]
    fn set_trove_status(id: Identity, status: Status) {
        require_caller_is_borrow_operations_contract();
        match storage.troves.get(id).try_read() {
            Some(trove) => {
                let mut new_trove = trove;
                new_trove.status = status;
                storage.troves.insert(id, new_trove);
            },
            None => {
                let mut trove = Trove::default();
                trove.status = status;
                storage.troves.insert(id, trove);
            }
        }
    }
    #[storage(read, write)]
    fn increase_trove_coll(id: Identity, coll: u64) -> u64 {
        require_caller_is_borrow_operations_contract();
        internal_increase_trove_coll(id, coll)
    }
    #[storage(read, write)]
    fn update_stake_and_total_stakes(id: Identity) -> u64 {
        require_caller_is_borrow_operations_contract();
        internal_update_stake_and_total_stakes(id)
    }
    #[storage(read, write)]
    fn increase_trove_debt(id: Identity, debt: u64) -> u64 {
        require_caller_is_borrow_operations_contract();
        internal_increase_trove_debt(id, debt)
    }
    #[storage(read, write)]
    fn decrease_trove_coll(id: Identity, value: u64) -> u64 {
        require_caller_is_borrow_operations_contract();
        internal_decrease_trove_coll(id, value)
    }
    #[storage(read, write)]
    fn decrease_trove_debt(id: Identity, value: u64) -> u64 {
        require_caller_is_borrow_operations_contract();
        internal_decrease_trove_debt(id, value)
    }
    #[storage(read, write)]
    fn add_trove_owner_to_array(id: Identity) -> u64 {
        require_caller_is_borrow_operations_contract();
        storage.trove_owners.push(id);
        let indx = storage.trove_owners.len() - 1;
        let mut trove = storage.troves.get(id).read();
        trove.array_index = indx;
        storage.troves.insert(id, trove);
        return indx;
    }
    #[storage(read)]
    fn get_trove_debt(id: Identity) -> u64 {
        let trove = storage.troves.get(id).read();
        return trove.debt;
    }
    #[storage(read)]
    fn get_trove_coll(id: Identity) -> u64 {
        let trove = storage.troves.get(id).read();
        return trove.coll;
    }
    #[storage(read, write)]
    fn close_trove(id: Identity) {
        require_caller_is_borrow_operations_contract();
        internal_close_trove(id, Status::ClosedByOwner);
    }
    #[storage(read, write)]
    fn remove_stake(id: Identity) {
        require_caller_is_borrow_operations_contract();
        internal_remove_stake(id);
    }
    #[storage(read)]
    fn get_trove_status(id: Identity) -> Status {
        match storage.troves.get(id).try_read() {
            Some(trove) => return trove.status,
            None => return Status::NonExistent,
        }
    }
    #[storage(read, write)]
    fn batch_liquidate_troves(
        borrowers: Vec<Identity>,
        upper_partial_hint: Identity,
        lower_partial_hint: Identity,
    ) {
        internal_batch_liquidate_troves(borrowers, upper_partial_hint, lower_partial_hint);
    }
    #[storage(read, write)]
    fn liquidate(
        id: Identity,
        upper_partial_hint: Identity,
        lower_partial_hint: Identity,
    ) {
        require_trove_is_active(id);
        let mut borrowers: Vec<Identity> = Vec::new();
        borrowers.push(id);
        internal_batch_liquidate_troves(borrowers, upper_partial_hint, lower_partial_hint);
    }
    #[storage(read, write)]
    fn update_trove_reward_snapshots(id: Identity) {
        require_caller_is_borrow_operations_contract();
        internal_update_trove_reward_snapshots(id);
    }
    #[storage(read)]
    fn get_pending_usdf_rewards(address: Identity) -> u64 {
        internal_get_pending_usdf_reward(address)
    }
    #[storage(read)]
    fn get_pending_asset_rewards(id: Identity) -> u64 {
        internal_get_pending_asset_reward(id)
    }
}
#[storage(read, write)]
fn internal_update_trove_reward_snapshots(id: Identity) {
    let reward_snapshot = RewardSnapshot {
        asset: storage.l_asset.read(),
        usdf_debt: storage.l_usdf.read(),
    };
    storage.reward_snapshots.insert(id, reward_snapshot);
}
#[storage(read, write)]
fn internal_apply_pending_rewards(borrower: Identity) {
    if (internal_has_pending_rewards(borrower)) {
        require_trove_is_active(borrower);
        let pending_asset = internal_get_pending_asset_reward(borrower);
        let pending_usdf = internal_get_pending_usdf_reward(borrower);
        let mut trove = storage.troves.get(borrower).read();
        trove.coll += pending_asset;
        trove.debt += pending_usdf;
        storage.troves.insert(borrower, trove);
        internal_update_trove_reward_snapshots(borrower);
        internal_move_pending_trove_rewards_to_active_pool(pending_asset, pending_usdf);
    }
}
#[storage(read, write)]
fn internal_close_trove(id: Identity, close_status: Status) {
    require(
        storage
            .lock_internal_close_trove
            .read() == false,
        "TroveManager: Internal close trove is locked",
    );
    storage.lock_internal_close_trove.write(true);
    require(
        close_status != Status::NonExistent && close_status != Status::Active,
        "TroveManager: Invalid status",
    );
    let asset_contract_cache = storage.asset_contract.read();
    let trove_owner_array_length = storage.trove_owners.len();
    let sorted_troves_contract_cache = storage.sorted_troves_contract.read();
    let sorted_troves = abi(SortedTroves, sorted_troves_contract_cache.into());
    require_more_than_one_trove_in_system(
        trove_owner_array_length,
        asset_contract_cache,
        sorted_troves_contract_cache,
    );
    let mut trove = storage.troves.get(id).read();
    trove.status = close_status;
    trove.coll = 0;
    trove.debt = 0;
    storage.troves.insert(id, trove);
    let mut rewards_snapshot = storage.reward_snapshots.get(id).read();
    rewards_snapshot.asset = 0;
    rewards_snapshot.usdf_debt = 0;
    storage.reward_snapshots.insert(id, rewards_snapshot);
    internal_remove_trove_owner(id, trove_owner_array_length);
    sorted_troves.remove(id, asset_contract_cache);
    storage.lock_internal_close_trove.write(false);
}
#[storage(read, write)]
fn internal_remove_trove_owner(_borrower: Identity, _trove_array_owner_length: u64) {
    let mut trove = storage.troves.get(_borrower).read();
    require(
        trove.status != Status::NonExistent && trove.status != Status::Active,
        "TroveManager: Trove does not exist",
    );
    let index = trove.array_index;
    let length = _trove_array_owner_length;
    let indx_last = length - 1;
    require(index <= indx_last, "TroveManager: Trove does not exist");
    let address_to_move = storage.trove_owners.get(indx_last).unwrap().read();
    let mut trove_to_move = storage.troves.get(address_to_move).read();
    trove_to_move.array_index = index;
    storage.troves.insert(address_to_move, trove_to_move);
    let _ = storage.trove_owners.swap_remove(index);
}
#[storage(read)]
fn require_trove_is_active(id: Identity) {
    let trove = storage.troves.get(id).read();
    require(
        trove.status == Status::Active,
        "TroveManager: Trove is not active",
    );
}
#[storage(read, write)]
fn internal_batch_liquidate_troves(
    borrowers: Vec<Identity>,
    upper_partial_hint: Identity,
    lower_partial_hint: Identity,
) {
    // Prevent reentrancy
    require(
        storage
            .lock_internal_batch_liquidate_troves
            .read() == false,
        "TroveManager: Internal batch liquidate troves is locked",
    );
    storage.lock_internal_batch_liquidate_troves.write(true);
    // Ensure there are borrowers to liquidate
    require(
        borrowers
            .len() > 0,
        "TroveManager: No borrowers to liquidate",
    );
    require_all_troves_unique(borrowers);
    require_all_troves_are_active(borrowers);
    require_all_troves_sorted_by_nicr(borrowers);

    // Initialize local variables and contracts
    let mut vars = LocalVariablesOuterLiquidationFunction::default();
    let oracle = abi(Oracle, storage.oracle_contract.read().into());
    let asset_contract_cache = storage.asset_contract.read();
    vars.price = oracle.get_price();
    let stability_pool = abi(StabilityPool, storage.stability_pool_contract.read().into());
    let total_usdf_in_sp = stability_pool.get_total_usdf_deposits();
    // Calculate totals for the batch liquidation
    let totals = internal_get_totals_from_batch_liquidate(
        vars.price,
        total_usdf_in_sp,
        borrowers,
        upper_partial_hint,
        lower_partial_hint,
    );
    // Ensure there is debt to liquidate
    require(
        totals
            .total_debt_in_sequence > 0,
        "TroveManager: No debt to liquidate",
    );
    // Offset debt and collateral with the Stability Pool
    stability_pool.offset(
        totals
            .total_debt_to_offset,
        totals
            .total_coll_to_send_to_sp,
        storage
            .asset_contract
            .read(),
    );
    let active_pool = abi(ActivePool, storage.active_pool_contract.read().into());
    // Handle collateral surplus if any
    if (totals.total_coll_surplus > 0) {
        active_pool.send_asset(
            Identity::ContractId(storage.coll_surplus_pool_contract.read()),
            totals
                .total_coll_surplus,
            asset_contract_cache,
        );
    }
    internal_update_system_snapshots_exclude_coll_remainder(totals.total_coll_gas_compensation);
    // Send gas compensation to the caller (liquidator)
    if (totals.total_coll_gas_compensation > 0) {
        active_pool.send_asset(
            msg_sender()
                .unwrap(),
            totals
                .total_coll_gas_compensation,
            asset_contract_cache,
        );
    }
    // Redistribute remaining debt and collateral
    internal_redistribute_debt_and_coll(
        totals
            .total_debt_to_redistribute,
        totals
            .total_coll_to_redistribute,
    );
    // Release the reentrancy lock
    storage.lock_internal_batch_liquidate_troves.write(false);
}
#[storage(read)]
fn require_caller_is_borrow_operations_contract() {
    let caller = msg_sender().unwrap();
    let borrow_operations_contract = Identity::ContractId(storage.borrow_operations_contract.read());
    require(
        caller == borrow_operations_contract,
        "TroveManager: Caller is not the Borrow Operations contract",
    );
}
#[storage(read)]
fn require_caller_is_protocol_manager_contract() {
    let caller = msg_sender().unwrap();
    let protocol_manager_contract = Identity::ContractId(storage.protocol_manager_contract.read());
    require(
        caller == protocol_manager_contract,
        "TroveManager: Caller is not the Protocol Manager contract",
    );
}
#[storage(read)]
fn require_caller_is_borrow_operations_contract_or_protocol_manager() {
    let caller = msg_sender().unwrap();
    let borrow_operations_contract = Identity::ContractId(storage.borrow_operations_contract.read());
    let protocol_manager_contract = Identity::ContractId(storage.protocol_manager_contract.read());
    require(
        caller == borrow_operations_contract || caller == protocol_manager_contract,
        "TroveManager: Caller is not the Borrow Operations or Protocol Manager contract",
    );
}
#[storage(read, write)]
fn internal_increase_trove_coll(id: Identity, coll: u64) -> u64 {
    let mut trove = storage.troves.get(id).read();
    trove.coll += coll;
    storage.troves.insert(id, trove);
    return trove.coll;
}
#[storage(read, write)]
fn internal_increase_trove_debt(id: Identity, debt: u64) -> u64 {
    let mut trove = storage.troves.get(id).read();
    trove.debt += debt;
    storage.troves.insert(id, trove);
    return trove.debt;
}
#[storage(read, write)]
fn internal_decrease_trove_coll(id: Identity, coll: u64) -> u64 {
    let mut trove = storage.troves.get(id).read();
    trove.coll -= coll;
    storage.troves.insert(id, trove);
    return trove.coll;
}
#[storage(read, write)]
fn internal_decrease_trove_debt(id: Identity, debt: u64) -> u64 {
    let mut trove = storage.troves.get(id).read();
    trove.debt -= debt;
    storage.troves.insert(id, trove);
    return trove.debt;
}
#[storage(read, write)]
fn internal_get_totals_from_batch_liquidate(
    price: u64,
    usdf_in_stability_pool: u64,
    borrowers: Vec<Identity>,
    upper_partial_hint: Identity,
    lower_partial_hint: Identity,
) -> LiquidationTotals {
    // Initialize variables for the liquidation sequence
    let mut vars = LocalVariablesLiquidationSequence::default();
    vars.remaining_usdf_in_stability_pool = usdf_in_stability_pool;
    let mut single_liquidation = LiquidationValues::default();
    let mut i = 0;
    let mut totals = LiquidationTotals::default();
    // Iterate through the list of borrowers
    while i < borrowers.len() {
        vars.borrower = borrowers.get(i).unwrap();
        // Calculate the Individual Collateralization Ratio (ICR) for the current borrower
        vars.icr = internal_get_current_icr(vars.borrower, price);
        // If the trove is undercollateralized (ICR < Minimum Collateralization Ratio), liquidate it
        if vars.icr < MCR {
            // Get the entire debt and collateral for the trove
            let position = internal_get_entire_debt_and_coll(vars.borrower);
            // Move any pending rewards to the active pool before liquidation
            internal_move_pending_trove_rewards_to_active_pool(position.pending_coll_rewards, position.pending_debt_rewards);
            // Calculate the values for offsetting debt and redistributing collateral
            single_liquidation = get_offset_and_redistribution_vals(
                position
                    .entire_trove_coll,
                position
                    .entire_trove_debt,
                vars.remaining_usdf_in_stability_pool,
                price,
            );
            // Apply the liquidation to the trove
            internal_apply_liquidation(
                vars.borrower,
                single_liquidation,
                upper_partial_hint,
                lower_partial_hint,
            );
            // Update the remaining USDF in the stability pool
            vars.remaining_usdf_in_stability_pool -= single_liquidation.debt_to_offset;
            // Add the results of this liquidation to the running totals
            totals = add_liquidation_vals_to_totals(totals, single_liquidation);
        } else {
            // If we've reached a trove that's not undercollateralized, we can stop the liquidation process
            break;
        }
        i += 1;
    }
    // Return the total results of all liquidations performed
    return totals;
}
#[storage(read)]
fn require_more_than_one_trove_in_system(
    trove_owner_array_length: u64,
    asset_contract: AssetId,
    sorted_troves_contract: ContractId,
) {
    let sorted_troves = abi(SortedTroves, sorted_troves_contract.into());
    let size = sorted_troves.get_size(asset_contract);
    require(
        trove_owner_array_length > 1 && size > 1,
        "TroveManager: There is only one trove in the system",
    );
}
#[storage(read)]
fn internal_get_current_icr(borrower: Identity, price: u64) -> u64 {
    let position = internal_get_entire_debt_and_coll(borrower);
    return fm_compute_cr(
        position
            .entire_trove_coll,
        position
            .entire_trove_debt,
        price,
    );
}
#[storage(read)]
fn internal_get_trove_stake(borrower: Identity) -> u64 {
    let trove = storage.troves.get(borrower).read();
    return trove.stake;
}
#[storage(read, write)]
fn internal_remove_stake(borrower: Identity) {
    let mut trove = storage.troves.get(borrower).read();
    storage
        .total_stakes
        .write(storage.total_stakes.read() - trove.stake);
    trove.stake = 0;
    storage.troves.insert(borrower, trove);
}
#[storage(read)]
fn internal_get_entire_debt_and_coll(borrower: Identity) -> EntireTroveDebtAndColl {
    let trove = storage.troves.get(borrower).try_read().unwrap_or(Trove::default());
    let coll = trove.coll;
    let debt = trove.debt;
    let pending_coll_rewards = internal_get_pending_asset_reward(borrower);
    let pending_debt_rewards = internal_get_pending_usdf_reward(borrower);
    return EntireTroveDebtAndColl {
        entire_trove_debt: debt + pending_debt_rewards,
        entire_trove_coll: coll + pending_coll_rewards,
        pending_debt_rewards,
        pending_coll_rewards,
    }
}
#[storage(read, write)]
fn internal_apply_liquidation(
    borrower: Identity,
    liquidation_values: LiquidationValues,
    upper_partial_hint: Identity,
    lower_partial_hint: Identity,
) {
    let asset_contract_cache = storage.asset_contract.read();
    // partial liquidation reinserted into sorted troves
    if (liquidation_values.is_partial_liquidation) {
        let mut trove = storage.troves.get(borrower).read();
        trove.coll = liquidation_values.remaining_trove_coll;
        trove.debt = liquidation_values.remaining_trove_debt;
        storage.troves.insert(borrower, trove);
        let _ = internal_update_stake_and_total_stakes(borrower);
        let _ = internal_update_trove_reward_snapshots(borrower);
        let new_ncr = fm_compute_nominal_cr(trove.coll, trove.debt);
        let sorted_troves_contract = abi(SortedTroves, storage.sorted_troves_contract.read().into());
        sorted_troves_contract.re_insert(
            borrower,
            new_ncr,
            upper_partial_hint,
            lower_partial_hint,
            asset_contract_cache,
        );

        // Add TroveUpdatedEvent for partial liquidation
        log(TrovePartialLiquidationEvent {
            borrower: borrower,
            remaining_debt: liquidation_values.remaining_trove_debt,
            remaining_collateral: liquidation_values.remaining_trove_coll,
        });
    } else {
        // liquidation of entire trove, sends the surplus to the coll surplus pool
        let coll_surplus_contract = abi(CollSurplusPool, storage.coll_surplus_pool_contract.read().into());
        internal_remove_stake(borrower);
        internal_close_trove(borrower, Status::ClosedByLiquidation);
        coll_surplus_contract.account_surplus(
            borrower,
            liquidation_values
                .coll_surplus,
            asset_contract_cache,
        );

        // Add TroveLiquidatedEvent for full liquidation
        log(TroveFullLiquidationEvent {
            borrower: borrower,
            debt: liquidation_values.entire_trove_debt,
            collateral: liquidation_values.entire_trove_coll,
        });
    }
}
#[storage(read, write)]
fn internal_redistribute_debt_and_coll(debt: u64, coll: u64) {
    let asset_contract_cache = storage.asset_contract.read();
    if (debt == 0) {
        return;
    }
    let asset_numerator: U128 = U128::from(coll) * U128::from(DECIMAL_PRECISION) + U128::from(storage.last_asset_error_redistribution.read());
    let usdf_numerator: U128 = U128::from(debt) * U128::from(DECIMAL_PRECISION) + U128::from(storage.last_usdf_error_redistribution.read());
    let asset_reward_per_unit_staked = asset_numerator / U128::from(storage.total_stakes.read());
    let usdf_reward_per_unit_staked = usdf_numerator / U128::from(storage.total_stakes.read());
    storage
        .last_asset_error_redistribution
        .write(
            (asset_numerator - (asset_reward_per_unit_staked * U128::from(storage.total_stakes.read())))
                .as_u64()
                .unwrap(),
        );
    storage
        .last_usdf_error_redistribution
        .write(
            (usdf_numerator - (usdf_reward_per_unit_staked * U128::from(storage.total_stakes.read())))
                .as_u64()
                .unwrap(),
        );
    storage
        .l_asset
        .write(storage.l_asset.read() + asset_reward_per_unit_staked.as_u64().unwrap());
    storage
        .l_usdf
        .write(storage.l_usdf.read() + usdf_reward_per_unit_staked.as_u64().unwrap());
    let active_pool = abi(ActivePool, storage.active_pool_contract.read().into());
    let default_pool = abi(DefaultPool, storage.default_pool_contract.read().into());
    active_pool.decrease_usdf_debt(debt, asset_contract_cache);
    default_pool.increase_usdf_debt(debt, asset_contract_cache);
    active_pool.send_asset_to_default_pool(coll, asset_contract_cache);
}
#[storage(read, write)]
fn internal_update_stake_and_total_stakes(address: Identity) -> u64 {
    let mut trove = storage.troves.get(address).read();
    let new_stake = internal_compute_new_stake(trove.coll);
    let old_stake = trove.stake;
    trove.stake = new_stake;
    storage.troves.insert(address, trove);
    let old_total_stakes = storage.total_stakes.read();
    storage
        .total_stakes
        .write(old_total_stakes + new_stake - old_stake);
    return new_stake;
}
#[storage(read)]
fn internal_get_pending_asset_reward(address: Identity) -> u64 {
    let snapshot_asset = storage.reward_snapshots.get(address).read().asset;
    let reward_per_unit_staked = storage.l_asset.read() - snapshot_asset;
    if (reward_per_unit_staked == 0
        || storage.troves.get(address).read().status != Status::Active)
    {
        return 0;
    }
    let stake = storage.troves.get(address).read().stake;
    let pending_asset_reward = fm_multiply_ratio(reward_per_unit_staked, stake, DECIMAL_PRECISION);
    return pending_asset_reward;
}
#[storage(read)]
fn internal_get_pending_usdf_reward(address: Identity) -> u64 {
    let snapshot_usdf = storage.reward_snapshots.get(address).read().usdf_debt;
    let reward_per_unit_staked = storage.l_usdf.read() - snapshot_usdf;
    if (reward_per_unit_staked == 0
        || storage.troves.get(address).read().status != Status::Active)
    {
        return 0;
    }
    let stake = storage.troves.get(address).read().stake;
    let pending_usdf_reward = fm_multiply_ratio(reward_per_unit_staked, stake, DECIMAL_PRECISION);
    return pending_usdf_reward;
}
#[storage(read)]
fn internal_has_pending_rewards(address: Identity) -> bool {
    if (storage.troves.get(address).read().status != Status::Active)
    {
        return false;
    }
    return (storage.reward_snapshots.get(address).read().asset < storage.l_asset.read());
}
#[storage(read)]
fn internal_move_pending_trove_rewards_to_active_pool(coll: u64, debt: u64) {
    if (coll == 0 && debt == 0) {
        return;
    }
    let asset_contract_cache = storage.asset_contract.read();
    let default_pool = abi(DefaultPool, storage.default_pool_contract.read().into());
    let active_pool = abi(ActivePool, storage.active_pool_contract.read().into());
    default_pool.decrease_usdf_debt(debt, asset_contract_cache);
    active_pool.increase_usdf_debt(debt, asset_contract_cache);
    default_pool.send_asset_to_active_pool(coll, asset_contract_cache);
}
#[storage(read)]
fn internal_get_entire_system_debt() -> u64 {
    let active_pool = abi(ActivePool, storage.active_pool_contract.read().into());
    let default_pool = abi(DefaultPool, storage.default_pool_contract.read().into());
    let asset_contract_cache = storage.asset_contract.read();
    return active_pool.get_usdf_debt(asset_contract_cache) + default_pool.get_usdf_debt(asset_contract_cache);
}
#[storage(read, write)]
fn internal_redeem_collateral_from_trove(
    borrower: Identity,
    max_usdf_amount: u64,
    price: u64,
    partial_redemption_hint: u64,
    upper_partial_hint: Identity,
    lower_partial_hint: Identity,
) -> SingleRedemptionValues {
    // Prevent reentrancy
    require(
        storage
            .lock_internal_redeem_collateral_from_trove
            .read() == false,
        "TroveManager: Internal redeem collateral from trove is locked",
    );
    storage
        .lock_internal_redeem_collateral_from_trove
        .write(true);
    let mut single_redemption_values = SingleRedemptionValues::default();
    let sorted_troves = abi(SortedTroves, storage.sorted_troves_contract.read().into());
    let asset_contract_cache = storage.asset_contract.read();
    // Get the trove details for the borrower
    let trove = storage.troves.get(borrower).read();
    // Calculate the amount of USDF to redeem (capped by max_usdf_amount or trove's debt)
    single_redemption_values.usdf_lot = fm_min(max_usdf_amount, trove.debt);
    // Calculate the corresponding amount of asset to redeem based on the current price
    single_redemption_values.asset_lot = fm_multiply_ratio(single_redemption_values.usdf_lot, DECIMAL_PRECISION, price);
    // Calculate the new debt and collateral amounts after redemption
    let new_debt = trove.debt - single_redemption_values.usdf_lot;
    let new_coll = trove.coll - single_redemption_values.asset_lot;
    // If the trove's debt is fully redeemed, close the trove
    if (new_debt == 0) {
        internal_remove_stake(borrower);
        internal_close_trove(borrower, Status::ClosedByRedemption);
        internal_redeem_close_trove(borrower, 0, new_coll);
    } else {
        // Calculate the new nominal collateralization ratio
        let new_nicr = fm_compute_nominal_cr(new_coll, new_debt);
        // If the new debt is below the minimum allowed, cancel the partial redemption
        if (new_debt < MIN_NET_DEBT) {
            single_redemption_values.cancelled_partial = true;
            storage
                .lock_internal_redeem_collateral_from_trove
                .write(false);
            return single_redemption_values;
        }
        // Re-insert the trove into the sorted list with its new NICR
        sorted_troves.re_insert(
            borrower,
            new_nicr,
            upper_partial_hint,
            lower_partial_hint,
            asset_contract_cache,
        );
        // Update the trove's debt and collateral in storage
        let mut trove = storage.troves.get(borrower).read();
        trove.debt = new_debt;
        trove.coll = new_coll;
        storage.troves.insert(borrower, trove);
        // Update the stake and total stakes
        internal_update_stake_and_total_stakes(borrower);
    }
    // Add RedemptionEvent before returning
    log(RedemptionEvent {
        borrower: borrower,
        usdf_amount: single_redemption_values.usdf_lot,
        collateral_amount: single_redemption_values.asset_lot,
        collateral_price: price,
    });
    storage
        .lock_internal_redeem_collateral_from_trove
        .write(false);
    return single_redemption_values;
}
#[storage(read, write)]
fn internal_redeem_close_trove(borrower: Identity, usdf_amount: u64, asset_amount: u64) {
    let asset_contract = storage.asset_contract.read();
    let coll_surplus_pool_contract = storage.coll_surplus_pool_contract.read();
    let usdf_contract = abi(SRC3, storage.usdf_contract.read().into());
    let active_pool = abi(ActivePool, storage.active_pool_contract.read().into());
    let coll_surplus_pool = abi(CollSurplusPool, coll_surplus_pool_contract.into());
    usdf_contract
        .burn {
            coins: usdf_amount,
            asset_id: storage.usdf_contract.read().bits(),
        }(SubId::zero(), usdf_amount);
    active_pool.decrease_usdf_debt(usdf_amount, asset_contract);
    coll_surplus_pool.account_surplus(borrower, asset_amount, asset_contract);
    active_pool.send_asset(
        Identity::ContractId(coll_surplus_pool_contract),
        asset_amount,
        asset_contract,
    );
}
fn require_all_troves_unique(borrowers: Vec<Identity>) {
    let mut outer_index = 0;
    while outer_index < borrowers.len() {
        let mut inner_index = outer_index + 1;
        while inner_index < borrowers.len() {
            require(
                borrowers
                    .get(outer_index)
                    .unwrap() != borrowers
                    .get(inner_index)
                    .unwrap(),
                "TroveManager: Duplicate borrower found",
            );
            inner_index += 1;
        }
        outer_index += 1;
    }
}
#[storage(read)]
fn require_all_troves_sorted_by_nicr(borrowers: Vec<Identity>) {
    let mut i = 0;
    while i < borrowers.len() {
        if i > 0 {
            require(
                internal_get_nominal_icr(borrowers.get(i).unwrap()) >= internal_get_nominal_icr(borrowers.get(i - 1).unwrap()),
                "TroveManager: Borrowers not sorted by nominal ICR",
            );
        }
        i += 1;
    }
}
#[storage(read)]
fn require_all_troves_are_active(borrowers: Vec<Identity>) {
    let mut i = 0;
    while i < borrowers.len() {
        require(
            storage
                .troves
                .get(borrowers.get(i).unwrap())
                .read()
                .status == Status::Active,
            "TroveManager: Trove is not active",
        );
        i += 1;
    }
}
#[storage(read)]
fn internal_get_nominal_icr(borrower: Identity) -> u64 {
    match storage.troves.get(borrower).try_read() {
        Some(trove) => {
            let position = internal_get_entire_debt_and_coll(borrower);
            return fm_compute_nominal_cr(position.entire_trove_coll, position.entire_trove_debt);
        }
        None => return fm_compute_nominal_cr(0, 0),
    }
}
// Calculate a new stake based on the snapshots of the totalStakes and totalCollateral taken at the last liquidation
#[storage(read)]
fn internal_compute_new_stake(coll: u64) -> u64 {
    let mut stake: u64 = 0;
    if storage.total_collateral_snapshot.read() == 0 {
        stake = coll;
    } else {
        /*
        * The following assert() holds true because:
        * - The system always contains >= 1 trove
        * - When we close or liquidate a trove, we redistribute the pending rewards, so if all troves were closed/liquidated,
        * rewards would’ve been emptied and totalCollateralSnapshot would be zero too.
        */
        require(
            storage
                .total_stakes_snapshot
                .read() > 0,
            "TroveManager: Total stakes snapshot is zero",
        );
        stake = fm_multiply_ratio(
            coll,
            storage
                .total_stakes_snapshot
                .read(),
            storage
                .total_collateral_snapshot
                .read(),
        );
    }
    return stake;
}
//
// Updates snapshots of system total stakes and total collateral, excluding a given collateral remainder from the calculation.
// Used in a liquidation sequence.
//
// The calculation excludes a portion of collateral that is in the ActivePool:
// the total collateral gas compensation from the liquidation sequence
//
// The collateral as compensation must be excluded as it is always sent out at the very end of the liquidation sequence.
//
#[storage(read, write)]
fn internal_update_system_snapshots_exclude_coll_remainder(coll_remainder: u64) {
    storage
        .total_stakes_snapshot
        .write(storage.total_stakes.read());

    let active_pool = abi(ActivePool, storage.active_pool_contract.read().into());
    let default_pool = abi(DefaultPool, storage.default_pool_contract.read().into());
    let active_pool_coll = active_pool.get_asset(storage.asset_contract.read());
    let liquidated_coll = default_pool.get_asset(storage.asset_contract.read());

    storage
        .total_collateral_snapshot
        .write(active_pool_coll - coll_remainder + liquidated_coll);
}
