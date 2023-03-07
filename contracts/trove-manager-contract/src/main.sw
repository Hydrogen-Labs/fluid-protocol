contract;

dep data_structures;
dep utils;

use utils::{calculate_liqudated_trove_values};
use data_structures::{
    LiquidationValues,
    LocalVariablesLiquidationSequence,
    LocalVariablesOuterLiquidationFunction,
    Trove,
};

use libraries::trove_manager_interface::{TroveManager};
use libraries::sorted_troves_interface::{SortedTroves};
use libraries::{MockOracle};
use libraries::data_structures::{Status};
use libraries::fluid_math::*;

use std::{
    address::Address,
    auth::msg_sender,
    block::{
        height,
        timestamp,
    },
    call_frames::{
        contract_id,
        msg_asset_id,
    },
    context::{
        msg_amount,
    },
    logging::log,
    storage::{
        StorageMap,
        StorageVec,
    },
    token::transfer,
};

const ZERO_B256 = 0x0000000000000000000000000000000000000000000000000000000000000000;

storage {
    sorted_troves_contract: ContractId = ContractId::from(ZERO_B256),
    borrow_operations_contract: ContractId = ContractId::from(ZERO_B256),
    oracle_contract: ContractId = ContractId::from(ZERO_B256),
    usdf: ContractId = ContractId::from(ZERO_B256),
    fpt_token: ContractId = ContractId::from(ZERO_B256),
    fpt_staking_contract: ContractId = ContractId::from(ZERO_B256),
    total_stakes: u64 = 0,
    total_stakes_snapshot: u64 = 0,
    total_collateral_snapshot: u64 = 0,
    f_asset: u64 = 0,
    f_usdf_debt: u64 = 0,
    last_asset_error_redistribution: u64 = 0,
    last_usdf_error_redistribution: u64 = 0,
    nominal_icr: StorageMap<Identity, u64> = StorageMap {},
    troves: StorageMap<Identity, Trove> = StorageMap {},
    trove_owners: StorageVec<Identity> = StorageVec {},
}

impl TroveManager for Contract {
    #[storage(read, write)]
    fn initialize(borrow_operations: ContractId, sorted_troves: ContractId) {
        storage.sorted_troves_contract = sorted_troves;
        storage.borrow_operations_contract = borrow_operations;
    }

    #[storage(read)]
    fn get_nominal_icr(id: Identity) -> u64 {
        let trove = storage.troves.get(id);

        return fm_compute_nominal_cr(trove.coll, trove.debt);
    }

    #[storage(read, write)]
    fn set_nominal_icr_and_insert(
        id: Identity,
        value: u64,
        prev_id: Identity,
        next_id: Identity,
    ) {
        // TODO Remove this function 
        storage.nominal_icr.insert(id, value);
        let sorted_troves_contract = abi(SortedTroves, storage.sorted_troves_contract.value);
        internal_increase_trove_coll(id, value);
        internal_increase_trove_debt(id, 1);

        sorted_troves_contract.insert(id, fm_compute_nominal_cr(value, 1), prev_id, next_id);
    }

    #[storage(read, write)]
    fn remove(id: Identity) {
        storage.nominal_icr.insert(id, 0);
        let sorted_troves_contract = abi(SortedTroves, storage.sorted_troves_contract.into());
        sorted_troves_contract.remove(id);
    }

    #[storage(read, write)]
    fn set_trove_status(id: Identity, status: Status) {
        require_caller_is_borrow_operations_contract();

        let mut trove = storage.troves.get(id);
        trove.status = status;
        storage.troves.insert(id, trove);
    }

    #[storage(read, write)]
    fn increase_trove_coll(id: Identity, coll: u64) -> u64 {
        require_caller_is_borrow_operations_contract();

        internal_increase_trove_coll(id, coll)
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

        let mut trove = storage.troves.get(id);
        trove.array_index = indx;
        storage.troves.insert(id, trove);

        return indx;
    }

    #[storage(read)]
    fn get_trove_debt(id: Identity) -> u64 {
        let trove = storage.troves.get(id);

        return trove.debt;
    }

    #[storage(read)]
    fn get_trove_coll(id: Identity) -> u64 {
        let trove = storage.troves.get(id);

        return trove.coll;
    }

    #[storage(read, write)]
    fn close_trove(id: Identity) {
        require_caller_is_borrow_operations_contract();

        internal_close_trove(id, Status::ClosedByOwner);
    }

    #[storage(read, write)]
    fn remove_stake(id: Identity) {}

    #[storage(read, write)]
    fn batch_liquidate_troves(borrowers: Vec<Identity>) {
        internal_batch_liquidate_troves(borrowers);
    }

    #[storage(read, write)]
    fn liquidate(id: Identity) {
        require_trove_is_active(id);

        let mut borrowers: Vec<Identity> = Vec::new();
        borrowers.push(id);

        internal_batch_liquidate_troves(borrowers);
    }

    #[storage(read, write)]
    fn liquidate_troves(num_troves: u64) {}
}

#[storage(read, write)]
fn internal_close_trove(id: Identity, close_status: Status) {
    require(close_status != Status::NonExistent || close_status != Status::Active, "Invalid status");

    let trove_owner_array_length = storage.trove_owners.len();

    require_more_than_one_trove_in_system(trove_owner_array_length);

    let mut trove = storage.troves.get(id);
    trove.status = close_status;
    trove.coll = 0;
    trove.debt = 0;
    storage.troves.insert(id, trove);

    // TODO Reward snapshot
    internal_remove_trove_owner(id, trove_owner_array_length);
    let sorted_troves_contract = abi(SortedTroves, storage.sorted_troves_contract.into());
    sorted_troves_contract.remove(id);
}

#[storage(read, write)]
fn internal_remove_trove_owner(_borrower: Identity, _trove_array_owner_length: u64) {
    let mut trove = storage.troves.get(_borrower);

    require(trove.status != Status::NonExistent && trove.status != Status::Active, "Trove does not exist");

    let index = trove.array_index;
    let length = _trove_array_owner_length;
    let indx_last = length - 1;

    require(index <= indx_last, "Trove does not exist");

    let address_to_move = storage.trove_owners.get(indx_last).unwrap();

    let mut trove_to_move = storage.troves.get(address_to_move);
    trove_to_move.array_index = index;
    storage.troves.insert(address_to_move, trove_to_move);

    let a = storage.trove_owners.swap_remove(index);
}

#[storage(read)]
fn require_trove_is_active(id: Identity) {
    let trove = storage.troves.get(id);
    require(trove.status == Status::Active, "Trove is not active");
}

#[storage(read, write)]
fn internal_batch_liquidate_troves(borrowers: Vec<Identity>) {
    require(borrowers.len() > 0, "No borrowers to liquidate");

    let mut vars = LocalVariablesOuterLiquidationFunction::default();
    let oracle = abi(MockOracle, storage.oracle_contract.into());

    vars.price = oracle.get_price();
    // TODO stabilit pool get usdf balance
}

#[storage(read)]
fn require_caller_is_borrow_operations_contract() {
    let caller = msg_sender().unwrap();
    let borrow_operations_contract = Identity::ContractId(storage.borrow_operations_contract);
    require(caller == borrow_operations_contract, "Caller is not the Borrow Operations contract");
}

#[storage(read, write)]
fn internal_increase_trove_coll(id: Identity, coll: u64) -> u64 {
    let mut trove = storage.troves.get(id);
    trove.coll += coll;
    storage.troves.insert(id, trove);

    return trove.coll;
}

#[storage(read, write)]
fn internal_increase_trove_debt(id: Identity, debt: u64) -> u64 {
    let mut trove = storage.troves.get(id);
    trove.debt += debt;
    storage.troves.insert(id, trove);

    return trove.debt;
}

#[storage(read, write)]
fn internal_decrease_trove_coll(id: Identity, coll: u64) -> u64 {
    let mut trove = storage.troves.get(id);
    trove.coll -= coll;
    storage.troves.insert(id, trove);

    return trove.coll;
}

#[storage(read, write)]
fn internal_decrease_trove_debt(id: Identity, debt: u64) -> u64 {
    let mut trove = storage.troves.get(id);
    trove.debt -= debt;
    storage.troves.insert(id, trove);

    return trove.debt;
}

#[storage(read)]
fn internal_get_totals_from_batch_liquidate(
    price: u64,
    usdf_in_stability_pool: u64,
    borrowers: Vec<Identity>,
) {
    let mut vars = LocalVariablesLiquidationSequence::default();
    let mut single_liquidation = LiquidationValues::default();
    let mut i = 0;
    while i < borrowers.len() {
        vars.borrower = borrowers.get(i).unwrap();
        vars.icr = internal_get_current_icr(vars.borrower, price);

        if vars.icr < MCR {        }
    }
}

#[storage(read)]
fn require_more_than_one_trove_in_system(trove_owner_array_length: u64) {
    let sorted_troves_contract = abi(SortedTroves, storage.sorted_troves_contract.into());
    let size = sorted_troves_contract.get_size();
    require(trove_owner_array_length > 1 && size > 1, "There is only one trove in the system");
}

#[storage(read)]
fn internal_get_current_icr(borrower: Identity, price: u64) -> u64 {
    let trove = storage.troves.get(borrower);
    let coll = trove.coll;
    let debt = trove.debt;

    return fm_compute_cr(coll, debt, price);
}
#[storage(read, write)]
fn internal_liquidate(
    borrower: Identity,
    usdf_in_stability_pool: u64,
    price: u64,
) -> LiquidationValues {
    let mut vars = LiquidationValues::default();

    let position = get_entire_debt_and_coll(borrower);
    vars.entire_trove_coll = position.0;
    vars.entire_trove_debt = position.1;

    // TODO move pending trove rewards to active pool
    // TODO Determine coll/usdf gas compensation 
    return vars;
}

#[storage(read)]
fn get_entire_debt_and_coll(borrower: Identity) -> (u64, u64) {
    let trove = storage.troves.get(borrower);
    let coll = trove.coll;
    let debt = trove.debt;

    // TODO Include pending USDF rewards
    // TODO Include pending ASSET rewards
    return (coll, debt);
}

#[storage(read)]
fn get_offset_and_redistribution_vals(coll: u64, debt: u64, usdf_in_stab_pool: u64, price: u64) {
    let mut debt_to_offset = 0;
    let mut coll_to_send_to_sp = 0;
    let mut debt_to_redistribute = 0;
    let mut coll_to_redistribute = 0;
    let mut is_partial_liquidation = false;

    let liquidated_position_vals = calculate_liqudated_trove_values(coll, debt, price);

    let liquidated_position_coll = liquidated_position_vals.0;
    let liquidated_position_debt = liquidated_position_vals.1;
    let is_partial_liquidation = liquidated_position_vals.2;

    if (usdf_in_stab_pool > 0) {   
        // If the Stability Pool doesnt have enough USDF to offset the entire debt, offset as much as possible
        if (liquidated_position_debt > usdf_in_stab_pool) {
            debt_to_offset = usdf_in_stab_pool;
        } else {
            // If the Stability Pool has enough USDF to offset the entire debt, offset the entire debt
            debt_to_offset = liquidated_position_debt;
        }
        // Send collateral to the Stability Pool proportional to the amount of debt offset
        coll_to_send_to_sp = liquidated_position_coll * debt_to_offset / liquidated_position_debt;
        // If stability pool doesn't have enough USDF to offset the entire debt, redistribute the remaining debt and collateral
        debt_to_redistribute = liquidated_position_debt - debt_to_offset;
        coll_to_redistribute = liquidated_position_coll - coll_to_send_to_sp;
    } else {
        debt_to_redistribute = liquidated_position_debt;
        coll_to_redistribute = liquidated_position_coll;
    }
}
