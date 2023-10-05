contract;

mod data_structures;

use ::data_structures::{AssetContracts, LocalVariables_AdjustTrove, LocalVariables_OpenTrove};

use libraries::trove_manager_interface::data_structures::{Status};
use libraries::active_pool_interface::{ActivePool};
use libraries::token_interface::{Token};
use libraries::usdf_token_interface::{USDFToken};
use libraries::trove_manager_interface::{TroveManager};
use libraries::sorted_troves_interface::{SortedTroves};
use libraries::fpt_staking_interface::{FPTStaking};
use libraries::coll_surplus_pool_interface::{CollSurplusPool};
use libraries::mock_oracle_interface::{MockOracle};
use libraries::borrow_operations_interface::{BorrowOperations};
use libraries::fluid_math::*;

use std::{
    auth::msg_sender,
    call_frames::{
        contract_id,
        msg_asset_id,
    },
    context::{
        msg_amount,
    },
    hash::Hash,
    logging::log,
    token::transfer,
};

storage {
    asset_contracts: StorageMap<AssetId, AssetContracts> = StorageMap::<AssetId, AssetContracts> {},
    valid_asset_ids: StorageMap<AssetId, bool> = StorageMap::<AssetId, bool> {},
    usdf_contract: ContractId = ContractId::from(ZERO_B256),
    fpt_staking_contract: ContractId = ContractId::from(ZERO_B256),
    coll_surplus_pool_contract: ContractId = ContractId::from(ZERO_B256),
    active_pool_contract: ContractId = ContractId::from(ZERO_B256),
    protocol_manager_contract: ContractId = ContractId::from(ZERO_B256),
    sorted_troves_contract: ContractId = ContractId::from(ZERO_B256),
    usdf_asset_id: AssetId = ZERO_B256,
    is_initialized: bool = false,
}

impl BorrowOperations for Contract {
    #[storage(read, write)]
    fn initialize(
        usdf_contract: ContractId,
        fpt_staking_contract: ContractId,
        protocol_manager: ContractId,
        coll_surplus_pool_contract: ContractId,
        active_pool_contract: ContractId,
        sorted_troves_contract: ContractId,
    ) {
        require(!storage.is_initialized.read(), "BorrowOperations: already initialized");

        storage.usdf_contract.write(usdf_contract);
        storage.fpt_staking_contract.write(fpt_staking_contract);
        storage.protocol_manager_contract.write(protocol_manager);
        storage.coll_surplus_pool_contract.write(coll_surplus_pool_contract);
        storage.active_pool_contract.write(active_pool_contract);
        storage.sorted_troves_contract.write(sorted_troves_contract);
        storage.usdf_asset_id.write(get_default_asset_id(usdf_contract));
        storage.is_initialized.write(true);
    }

    #[storage(read, write)]
    fn add_asset(
        asset_contract: AssetId,
        trove_manager_contract: ContractId,
        oracle_contract: ContractId,
    ) {
        require_is_protocol_manager();
        let asset_contracts = AssetContracts {
            trove_manager: trove_manager_contract,
            oracle: oracle_contract,
        };

        storage.valid_asset_ids.insert(asset_contract, true);
        storage.asset_contracts.insert(asset_contract, asset_contracts);
    }

    // --- Borrower Trove Operations ---
    #[storage(read, write), payable]
    fn open_trove(usdf_amount: u64, upper_hint: Identity, lower_hint: Identity) {
        require_valid_asset_id();
        let asset_contract = msg_asset_id();
        let asset_contracts = storage.asset_contracts.get(asset_contract).read();
        let usdf_contract = storage.usdf_contract.read();
        let fpt_staking_contract = storage.fpt_staking_contract.read();
        let active_pool_contract = storage.active_pool_contract.read();
        let sorted_troves_contract = storage.sorted_troves_contract.read();

        let oracle = abi(MockOracle, asset_contracts.oracle.value);
        let trove_manager = abi(TroveManager, asset_contracts.trove_manager.value);
        let sorted_troves = abi(SortedTroves, sorted_troves_contract.value);
        let usdf = abi(Token, storage.usdf_contract.read().value);

        let mut vars = LocalVariables_OpenTrove::new();
        let sender = msg_sender().unwrap();
        vars.net_debt = usdf_amount;
        vars.price = oracle.get_price();

        require_trove_is_not_active(sender, asset_contracts.trove_manager);
        vars.usdf_fee = internal_trigger_borrowing_fee(vars.net_debt, usdf_contract, fpt_staking_contract);

        vars.net_debt += vars.usdf_fee;

        require_at_least_min_net_debt(vars.net_debt);

        vars.icr = fm_compute_cr(msg_amount(), vars.net_debt, vars.price);
        vars.nicr = fm_compute_nominal_cr(msg_amount(), vars.net_debt);

        require_at_least_mcr(vars.icr);


        // Set the trove struct's properties
        trove_manager.set_trove_status(sender, Status::Active);

        let _ = trove_manager.increase_trove_coll(sender, msg_amount());
        let _ = trove_manager.increase_trove_debt(sender, vars.net_debt);

        trove_manager.update_trove_reward_snapshots(sender);
        let _ = trove_manager.update_stake_and_total_stakes(sender);

        sorted_troves.insert(sender, vars.nicr, upper_hint, lower_hint, asset_contract);
        vars.array_index = trove_manager.add_trove_owner_to_array(sender);


        // Move the ether to the Active Pool, and mint the USDF to the borrower
        internal_active_pool_add_coll(msg_amount(), asset_contract, active_pool_contract);
        internal_withdraw_usdf(sender, usdf_amount, vars.net_debt, active_pool_contract, usdf_contract, asset_contract);
    }

    #[storage(read, write), payable]
    fn add_coll(upper_hint: Identity, lower_hint: Identity) {
        require_valid_asset_id();
        internal_adjust_trove(msg_sender().unwrap(), msg_amount(), 0, 0, false, upper_hint, lower_hint, msg_asset_id());
    }

    #[storage(read, write)]
    fn withdraw_coll(
        amount: u64,
        upper_hint: Identity,
        lower_hint: Identity,
        asset_contract: AssetId,
    ) {
        internal_adjust_trove(msg_sender().unwrap(), 0, amount, 0, false, upper_hint, lower_hint, asset_contract);
    }

    #[storage(read, write)]
    fn withdraw_usdf(
        amount: u64,
        upper_hint: Identity,
        lower_hint: Identity,
        asset_contract: AssetId,
    ) {
        internal_adjust_trove(msg_sender().unwrap(), 0, 0, amount, true, upper_hint, lower_hint, asset_contract);
    }

    #[storage(read, write), payable]
    fn repay_usdf(
        upper_hint: Identity,
        lower_hint: Identity,
        asset_contract: AssetId,
    ) {
        require_valid_usdf_id(msg_asset_id());

        internal_adjust_trove(msg_sender().unwrap(), 0, 0, msg_amount(), false, upper_hint, lower_hint, asset_contract);
    }

    #[storage(read, write), payable]
    fn close_trove(asset_contract: AssetId) {
        let asset_contracts_cache = storage.asset_contracts.get(asset_contract).read();

        let usdf_contract_cache = storage.usdf_contract.read();
        let active_pool_contract_cache = storage.active_pool_contract.read();
        let trove_manager = abi(TroveManager, asset_contracts_cache.trove_manager.value);
        let active_pool = abi(ActivePool, active_pool_contract_cache.value);
        let oracle = abi(MockOracle, asset_contracts_cache.oracle.value);
        let borrower = msg_sender().unwrap();

        require_trove_is_active(borrower, asset_contracts_cache.trove_manager);
        let price = oracle.get_price();
        trove_manager.apply_pending_rewards(borrower);

        let coll = trove_manager.get_trove_coll(borrower);
        let debt = trove_manager.get_trove_debt(borrower);

        if debt > 0 {
            require_valid_usdf_id(msg_asset_id());
            require(debt <= msg_amount(), "BorrowOperations: cannot close trove with insufficient usdf balance");
        }

        trove_manager.close_trove(borrower);
        trove_manager.remove_stake(borrower);
        internal_repay_usdf(debt, active_pool_contract_cache, usdf_contract_cache, asset_contract);
        active_pool.send_asset(borrower, coll, asset_contract);

        if (debt < msg_amount()) {
            let excess_usdf_returned = msg_amount() - debt;
            transfer(borrower, storage.usdf_asset_id.read(), excess_usdf_returned);
        }
    }

    #[storage(read)]
    fn claim_collateral(asset: AssetId) {
        let coll_surplus = abi(CollSurplusPool, storage.coll_surplus_pool_contract.read().value);
        coll_surplus.claim_coll(msg_sender().unwrap(), asset);
    }
}

#[storage(read)]
fn internal_trigger_borrowing_fee(
    usdf_amount: u64,
    usdf_contract: ContractId,
    fpt_staking_contract: ContractId,
) -> u64 {
    let usdf = abi(USDFToken, usdf_contract.value);
    let fpt_staking = abi(FPTStaking, fpt_staking_contract.value);

    let usdf_fee = fm_compute_borrow_fee(usdf_amount);
    // Mint usdf to fpt staking contract
    usdf.mint(usdf_fee, Identity::ContractId(fpt_staking_contract));
    //increase fpt staking rewards
    fpt_staking.increase_f_usdf(usdf_fee);

    return usdf_fee
}

#[storage(read)]
fn internal_adjust_trove(
    borrower: Identity,
    asset_coll_added: u64,
    coll_withdrawal: u64,
    usdf_change: u64,
    is_debt_increase: bool,
    upper_hint: Identity,
    lower_hint: Identity,
    asset: AssetId,
) {
    let asset_contracts_cache = storage.asset_contracts.get(asset).read();
    let usdf_contract_cache = storage.usdf_contract.read();
    let fpt_staking_contract_cache = storage.fpt_staking_contract.read();
    let active_pool_contract_cache = storage.active_pool_contract.read();
    let sorted_troves_contract_cache = storage.sorted_troves_contract.read();

    let oracle = abi(MockOracle, asset_contracts_cache.oracle.value);
    let trove_manager = abi(TroveManager, asset_contracts_cache.trove_manager.value);
    let sorted_troves = abi(SortedTroves, sorted_troves_contract_cache.value);

    let price = oracle.get_price();
    let mut vars = LocalVariables_AdjustTrove::new();

    if is_debt_increase {
        require_non_zero_debt_change(usdf_change);
    }
    require_trove_is_active(borrower, asset_contracts_cache.trove_manager);
    require_singular_coll_change(asset_coll_added, coll_withdrawal);
    require_non_zero_adjustment(asset_coll_added, coll_withdrawal, usdf_change);

    trove_manager.apply_pending_rewards(borrower);

    let pos_res = internal_get_coll_change(asset_coll_added, coll_withdrawal);
    vars.coll_change = pos_res.0;
    vars.is_coll_increase = pos_res.1;

    vars.net_debt_change = usdf_change;
    if is_debt_increase {
        vars.usdf_fee = internal_trigger_borrowing_fee(vars.net_debt_change, usdf_contract_cache, fpt_staking_contract_cache);

        vars.net_debt_change = vars.net_debt_change + vars.usdf_fee;
    }

    vars.debt = trove_manager.get_trove_debt(borrower);
    vars.coll = trove_manager.get_trove_coll(borrower);

    vars.old_icr = fm_compute_cr(vars.coll, vars.debt, price);
    vars.new_icr = internal_get_new_icr_from_trove_change(vars.coll, vars.debt, vars.coll_change, vars.is_coll_increase, vars.net_debt_change, is_debt_increase, price);

    require(coll_withdrawal <= vars.coll, "Cannot withdraw more than the Trove's collateral");

    require_at_least_mcr(vars.new_icr);

    // TODO if debt increase and usdf change > 0 
    // TODO if debt increase and usdf change > 0 
    if !is_debt_increase && usdf_change > 0 {
        require_at_least_min_net_debt(vars.debt - vars.net_debt_change);
    }

    let new_position_res = internal_update_trove_from_adjustment(borrower, vars.coll_change, vars.is_coll_increase, vars.net_debt_change, is_debt_increase, asset_contracts_cache.trove_manager);

    let _ = trove_manager.update_stake_and_total_stakes(borrower);
    let new_nicr = internal_get_new_nominal_icr_from_trove_change(vars.coll, vars.debt, vars.coll_change, vars.is_coll_increase, vars.net_debt_change, is_debt_increase);
    sorted_troves.re_insert(borrower, new_nicr, upper_hint, lower_hint, asset);

    internal_move_usdf_and_asset_from_adjustment(borrower, vars.coll_change, vars.is_coll_increase, usdf_change, is_debt_increase, vars.net_debt_change, asset, active_pool_contract_cache, usdf_contract_cache);
}

#[storage(read)]
fn require_is_protocol_manager() {
    let protocol_manager = Identity::ContractId(storage.protocol_manager_contract.read());
    require(msg_sender().unwrap() == protocol_manager, "Caller is not the protocol manager");
}

#[storage(read)]
fn require_trove_is_not_active(borrower: Identity, trove_manager: ContractId) {
    let trove_manager = abi(TroveManager, trove_manager.value);
    let status = trove_manager.get_trove_status(borrower);
    require(status != Status::Active, "BorrowOperations: User already has an active Trove");
}

#[storage(read)]
fn require_trove_is_active(borrower: Identity, trove_manage_contract: ContractId) {
    let trove_manager = abi(TroveManager, trove_manage_contract.value);
    let status = trove_manager.get_trove_status(borrower);
    require(status == Status::Active, "BorrowOperations: User does not have an active Trove");
}

#[storage(read)]
fn require_non_zero_adjustment(asset_amount: u64, coll_withdrawl: u64, usdf_change: u64) {
    require(asset_amount > 0 || coll_withdrawl > 0 || usdf_change > 0, "BorrowOperations: coll withdrawal and debt change must be greater than 0");
}

fn require_at_least_min_net_debt(_net_debt: u64) {
    require(_net_debt > MIN_NET_DEBT, "BorrowOperations: net debt must be greater than 0");
}

fn require_non_zero_debt_change(debt_change: u64) {
    require(debt_change > 0, "BorrowOperations: debt change must be greater than 0");
}

fn require_at_least_mcr(icr: u64) {
    require(icr > MCR, "Minimum collateral ratio not met");
}

#[storage(read)]
fn require_singular_coll_change(coll_added_amount: u64, coll_withdrawl: u64) {
    require(coll_withdrawl == 0 || 0 == coll_added_amount, "BorrowOperations: collateral change must be 0 or equal to the amount sent");
}

#[storage(read)]
fn require_valid_asset_id() {
    require(storage.valid_asset_ids.get(msg_asset_id()).read(), "Invalid asset being transfered");
}

#[storage(read)]
fn require_valid_usdf_id(recieved_asset: AssetId) {
    require(recieved_asset == storage.usdf_asset_id.read(), "Invalid asset being transfered");
}

#[storage(read)]
fn internal_withdraw_usdf(
    recipient: Identity,
    amount: u64,
    net_debt_increase: u64,
    active_pool_contract: ContractId,
    usdf_contract: ContractId,
    asset_contract: AssetId,
) {
    let active_pool = abi(ActivePool, active_pool_contract.value);
    let usdf = abi(USDFToken, usdf_contract.value);

    active_pool.increase_usdf_debt(net_debt_increase, asset_contract);
    usdf.mint(amount, recipient);
}

fn internal_get_coll_change(_coll_recieved: u64, _requested_coll_withdrawn: u64) -> (u64, bool) {
    if (_coll_recieved != 0) {
        return (_coll_recieved, true);
    } else {
        return (_requested_coll_withdrawn, false);
    }
}

fn internal_get_new_icr_from_trove_change(
    coll: u64,
    debt: u64,
    coll_change: u64,
    is_coll_increase: bool,
    debt_change: u64,
    is_debt_increase: bool,
    price: u64,
) -> u64 {
    let new_position = internal_get_new_trove_amounts(coll, debt, coll_change, is_coll_increase, debt_change, is_debt_increase);

    let new_icr = fm_compute_cr(new_position.0, new_position.1, price);
    return new_icr;
}

fn internal_get_new_nominal_icr_from_trove_change(
    coll: u64,
    debt: u64,
    coll_change: u64,
    is_coll_increase: bool,
    debt_change: u64,
    is_debt_increase: bool,
) -> u64 {
    let new_position = internal_get_new_trove_amounts(coll, debt, coll_change, is_coll_increase, debt_change, is_debt_increase);
    let new_icr = fm_compute_nominal_cr(new_position.0, new_position.1);

    return new_icr;
}

#[storage(read)]
fn internal_update_trove_from_adjustment(
    borrower: Identity,
    coll_change: u64,
    is_coll_increase: bool,
    debt_change: u64,
    is_debt_increase: bool,
    trove_manager: ContractId,
) -> (u64, u64) {
    let trove_manager = abi(TroveManager, trove_manager.value);
    let mut new_coll = 0;
    let mut new_debt = 0;

    if is_coll_increase {
        new_coll = trove_manager.increase_trove_coll(borrower, coll_change);
    } else {
        new_coll = trove_manager.decrease_trove_coll(borrower, coll_change);
    }

    if is_debt_increase {
        new_debt = trove_manager.increase_trove_debt(borrower, debt_change);
    } else {
        new_debt = trove_manager.decrease_trove_debt(borrower, debt_change);
    }

    return (new_coll, new_debt);
}

fn internal_get_new_trove_amounts(
    coll: u64,
    debt: u64,
    coll_change: u64,
    is_coll_increase: bool,
    debt_change: u64,
    is_debt_increase: bool,
) -> (u64, u64) {
    let new_coll = if is_coll_increase {
        coll + coll_change
    } else {
        coll - coll_change
    };

    let new_debt = if is_debt_increase {
        debt + debt_change
    } else {
        debt - debt_change
    };

    return (new_coll, new_debt);
}

#[storage(read)]
fn internal_active_pool_add_coll(coll_change: u64, asset: AssetId, active_pool: ContractId) {
    let active_pool = abi(ActivePool, active_pool.value);

    active_pool.recieve {
        coins: coll_change,
        asset_id: asset,
    }();
}

#[storage(read)]
fn internal_repay_usdf(
    usdf_amount: u64,
    active_pool_contract: ContractId,
    usdf_contract: ContractId,
    asset_contract: AssetId,
) {
    let active_pool = abi(ActivePool, active_pool_contract.value);
    let usdf = abi(USDFToken, usdf_contract.value);

    usdf.burn {
        coins: usdf_amount,
        asset_id: usdf_contract.value,
    }();

    active_pool.decrease_usdf_debt(usdf_amount, asset_contract);
}

#[storage(read)]
fn internal_move_usdf_and_asset_from_adjustment(
    borrower: Identity,
    coll_change: u64,
    is_coll_increase: bool,
    usdf_change: u64,
    is_debt_increase: bool,
    net_debt_change: u64,
    asset: AssetId,
    active_pool_contract: ContractId,
    usdf_contract: ContractId,
) {
    let active_pool = abi(ActivePool, active_pool_contract.value);

    if coll_change > 0 {
        if is_coll_increase {
            internal_active_pool_add_coll(coll_change, asset, active_pool_contract);
        } else {
            active_pool.send_asset(borrower, coll_change, asset);
        }
    }

    if usdf_change > 0 {
        if is_debt_increase {
            internal_withdraw_usdf(borrower, usdf_change, net_debt_change, active_pool_contract, usdf_contract, asset);
        } else {
            internal_repay_usdf(usdf_change, active_pool_contract, usdf_contract, asset);
        }
    }
}
