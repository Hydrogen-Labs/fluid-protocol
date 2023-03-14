contract;

dep data_structures;

use data_structures::{LocalVariables_AdjustTrove, LocalVariables_OpenTrove};

use libraries::data_structures::{Status};
use libraries::active_pool_interface::{ActivePool};
use libraries::token_interface::{Token};
use libraries::trove_manager_interface::{TroveManager};
use libraries::sorted_troves_interface::{SortedTroves};
use libraries::{MockOracle};
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
    logging::log,
    token::transfer,
};

const ZERO_B256 = 0x0000000000000000000000000000000000000000000000000000000000000000;

storage {
    trove_manager_contract: ContractId = ContractId::from(ZERO_B256),
    sorted_troves_contract: ContractId = ContractId::from(ZERO_B256),
    oracle_contract: ContractId = ContractId::from(ZERO_B256),
    active_pool_contract: ContractId = ContractId::from(ZERO_B256),
    asset_contract: ContractId = ContractId::from(ZERO_B256),
    usdf_contract: ContractId = ContractId::from(ZERO_B256),
    stability_pool_contract: ContractId = ContractId::from(ZERO_B256),
    fpt_staking_contract: ContractId = ContractId::from(ZERO_B256),
}

impl BorrowOperations for Contract {
    #[storage(read, write)]
    fn initialize(
        trove_manager_contract: ContractId,
        sorted_troves_contract: ContractId,
        oracle_contract: ContractId,
        asset_contract: ContractId,
        usdf_contract: ContractId,
        fpt_staking_contract: ContractId,
        active_pool_contract: ContractId,
    ) {
        require(storage.trove_manager_contract.value == ZERO_B256, "BorrowOperations: contract is already initialized");

        storage.trove_manager_contract = trove_manager_contract;
        storage.sorted_troves_contract = sorted_troves_contract;
        storage.oracle_contract = oracle_contract;
        storage.asset_contract = asset_contract;
        storage.usdf_contract = usdf_contract;
        storage.fpt_staking_contract = fpt_staking_contract;
        storage.active_pool_contract = active_pool_contract;
    }

    #[storage(read, write)]
    fn open_trove(
        _usdf_amount: u64,
        _upper_hint: Identity,
        _lower_hint: Identity,
    ) {
        require_valid_asset_id();

        let oracle = abi(MockOracle, storage.oracle_contract.value);
        let trove_manager = abi(TroveManager, storage.trove_manager_contract.value);
        let sorted_troves = abi(SortedTroves, storage.sorted_troves_contract.value);
        let usdf = abi(Token, storage.usdf_contract.value);

        let mut vars = LocalVariables_OpenTrove::new();

        vars.net_debt = _usdf_amount;
        vars.price = oracle.get_price();

        // TODO Require Trove is not active / exists
        vars.usdf_fee = internal_trigger_borrowing_fee();
        vars.net_debt = vars.net_debt + vars.usdf_fee;

        require_at_least_min_net_debt(vars.net_debt);

        // ICR is based on the composite debt, i.e. the requested usdf amount + usdf borrowing fee + usdf gas comp.
        require(vars.net_debt > 0, "BorrowOperations: composite debt must be greater than 0");

        let sender = msg_sender().unwrap();

        vars.icr = fm_compute_cr(msg_amount(), vars.net_debt, vars.price);
        vars.nicr = fm_compute_nominal_cr(msg_amount(), vars.net_debt);

        require_at_least_mcr(vars.icr);

        trove_manager.set_trove_status(sender, Status::Active);
        trove_manager.increase_trove_coll(sender, msg_amount());
        trove_manager.increase_trove_debt(sender, vars.net_debt);

        trove_manager.update_trove_reward_snapshots(sender);
        let _ = trove_manager.update_stake_and_total_stakes(sender);

        sorted_troves.insert(sender, vars.nicr, _upper_hint, _lower_hint);
        vars.array_index = trove_manager.add_trove_owner_to_array(sender);

        internal_active_pool_add_coll(msg_amount());
        withdraw_usdf(sender, _usdf_amount, _usdf_amount);
    }

    #[storage(read, write)]
    fn add_coll(_upper_hint: Identity, _lower_hint: Identity) {
        require_valid_asset_id();

        internal_adjust_trove(msg_sender().unwrap(), msg_amount(), 0, 0, false, _upper_hint, _lower_hint);
    }

    #[storage(read, write)]
    fn withdraw_coll(amount: u64, upper_hint: Identity, lower_hint: Identity) {
        internal_adjust_trove(msg_sender().unwrap(), 0, amount, 0, false, upper_hint, lower_hint);
    }

    #[storage(read, write)]
    fn move_asset_gain_to_trove(id: Identity, upper_hint: Identity, lower_hint: Identity) {
        require_caller_is_stability_pool();
        require_valid_asset_id();

        internal_adjust_trove(id, msg_amount(), 0, 0, false, upper_hint, lower_hint);
    }

    #[storage(read, write)]
    fn withdraw_usdf(amount: u64, upper_hint: Identity, lower_hint: Identity) {
        internal_adjust_trove(msg_sender().unwrap(), 0, 0, amount, true, upper_hint, lower_hint);
    }

    #[storage(read, write)]
    fn repay_usdf(upper_hint: Identity, lower_hint: Identity) {
        require_valid_usdf_id();

        internal_adjust_trove(msg_sender().unwrap(), 0, 0, msg_amount(), false, upper_hint, lower_hint);
    }

    #[storage(read, write)]
    fn close_trove() {
        let trove_manager = abi(TroveManager, storage.trove_manager_contract.value);
        let active_pool = abi(ActivePool, storage.active_pool_contract.value);
        let oracle = abi(MockOracle, storage.oracle_contract.value);

        // TODO require trove is active
        let price = oracle.get_price();

        // TODO trove manager apply pending rewards
        let coll = trove_manager.get_trove_coll(msg_sender().unwrap());
        let debt = trove_manager.get_trove_debt(msg_sender().unwrap());

        if debt > 0 {
            require_valid_usdf_id();
            require(debt <= msg_amount(), "BorrowOperations: cannot close trove with insufficient usdf balance");
        }

        trove_manager.close_trove(msg_sender().unwrap());
        trove_manager.remove_stake(msg_sender().unwrap());
        internal_repay_usdf(debt);
        active_pool.send_asset(msg_sender().unwrap(), coll);

        // TODO refund excess usdf
    }

    #[storage(read, write)]
    fn adjust_trove(
        coll_withdrawl: u64,
        debt_change: u64,
        is_debt_increase: bool,
        upper_hint: Identity,
        lower_hint: Identity,
    ) {}

        // Since you cannot attach two different assets to a single transaction, 
        // we need to check which asset is being used, probably will remove this function
    #[storage(read, write)]
    fn claim_collateral() {}

    #[storage(read, write)]
    fn get_composite_debt(id: Identity) -> u64 {
        return 0
    }
}

fn internal_trigger_borrowing_fee() -> u64 {
    // TODO
    return 0
}

// function _adjustTrove(address _borrower, uint _collWithdrawal, uint _usdfChange, bool _isDebtIncrease, address _upperHint, address _lowerHint, uint _maxFeePercentage) internal {
#[storage(read)]
fn internal_adjust_trove(
    _borrower: Identity,
    _asset_coll_added: u64,
    _coll_withdrawal: u64,
    _usdf_change: u64,
    _is_debt_increase: bool,
    _upper_hint: Identity,
    _lower_hint: Identity,
) {
    let oracle = abi(MockOracle, storage.oracle_contract.value);
    let trove_manager = abi(TroveManager, storage.trove_manager_contract.value);
    let sorted_troves = abi(SortedTroves, storage.sorted_troves_contract.value);
    let price = oracle.get_price();

    let mut vars = LocalVariables_AdjustTrove::new();

    if _is_debt_increase {
        require_non_zero_debt_change(_usdf_change);
    }

    require_singular_coll_change(_asset_coll_added, _coll_withdrawal);
    require_non_zero_adjustment(_asset_coll_added, _coll_withdrawal, _usdf_change);

    trove_manager.apply_pending_rewards(_borrower);

    let pos_res = internal_get_coll_change(_asset_coll_added, _coll_withdrawal);
    vars.coll_change = pos_res.0;
    vars.is_coll_increase = pos_res.1;

    vars.net_debt_change = _usdf_change;

    if _is_debt_increase {
        vars.usdf_fee = internal_trigger_borrowing_fee();
        vars.net_debt_change = vars.net_debt_change + vars.usdf_fee;
    }

    vars.debt = trove_manager.get_trove_debt(_borrower);
    vars.coll = trove_manager.get_trove_coll(_borrower);

    vars.old_icr = fm_compute_cr(vars.coll, vars.debt, price);
    vars.new_icr = internal_get_new_icr_from_trove_change(vars.coll, vars.debt, vars.coll_change, vars.is_coll_increase, vars.net_debt_change, _is_debt_increase, price);

    require(_coll_withdrawal <= vars.coll, "Cannot withdraw more than the Trove's collateral");

    require_at_least_mcr(vars.new_icr);
    // TODO require valid adjustment in current mode or leave same if no recovery mode
    // TODO if debt increase and usdf change > 0 
    if !_is_debt_increase {
        require_at_least_min_net_debt(vars.debt - vars.net_debt_change);
    }

    let new_position_res = internal_update_trove_from_adjustment(_borrower, vars.coll_change, vars.is_coll_increase, vars.net_debt_change, _is_debt_increase);

    let _ = trove_manager.update_stake_and_total_stakes(_borrower);
    let new_nicr = internal_get_new_nominal_icr_from_trove_change(vars.coll, vars.debt, vars.coll_change, vars.is_coll_increase, vars.net_debt_change, _is_debt_increase);
    sorted_troves.re_insert(_borrower, new_nicr, _upper_hint, _lower_hint);

    internal_move_usdf_and_asset_from_adjustment(_borrower, vars.coll_change, vars.is_coll_increase, _usdf_change, _is_debt_increase, vars.net_debt_change);
}

#[storage(read)]
fn require_caller_is_stability_pool() {
    require(msg_sender().unwrap() == Identity::ContractId(storage.stability_pool_contract), "BorrowOperations: Caller is not Stability Pool");
}

#[storage(read)]
fn require_non_zero_adjustment(asset_amount: u64, _coll_withdrawl: u64, _usdf_change: u64) {
    require(asset_amount > 0 || _coll_withdrawl > 0 || _usdf_change > 0, "BorrowOperations: coll withdrawal and debt change must be greater than 0");
}

fn require_at_least_min_net_debt(_net_debt: u64) {
    require(_net_debt > MIN_NET_DEBT, "BorrowOperations: net debt must be greater than 0");
}

fn require_non_zero_debt_change(_debt_change: u64) {
    require(_debt_change > 0, "BorrowOperations: debt change must be greater than 0");
}

fn require_at_least_mcr(icr: u64) {
    require(icr > MCR, "Minimum collateral ratio not met");
}

fn require_valid_max_fee_percentage(_max_fee_percentage: u64) {
    require(_max_fee_percentage < DECIMAL_PRECISION, "BorrowOperations: max fee percentage must be less than 100");
}

#[storage(read)]
fn require_singular_coll_change(_coll_added_amount: u64, _coll_withdrawl: u64) {
    require(_coll_withdrawl == 0 || 0 == _coll_added_amount, "BorrowOperations: collateral change must be 0 or equal to the amount sent");
}

#[storage(read)]
fn require_valid_asset_id() {
    require(msg_asset_id() == storage.asset_contract, "Invalid asset being transfered");
}

#[storage(read)]
fn require_valid_usdf_id() {
    require(msg_asset_id() == storage.usdf_contract, "Invalid asset being transfered");
}

#[storage(read)]
fn withdraw_usdf(recipient: Identity, amount: u64, net_debt_increase: u64) {
    let active_pool = abi(ActivePool, storage.active_pool_contract.value);
    let usdf = abi(Token, storage.usdf_contract.value);

    active_pool.increase_usdf_debt(net_debt_increase);
    usdf.mint_to_id(amount, recipient);
}

fn internal_get_coll_change(_coll_recieved: u64, _requested_coll_withdrawn: u64) -> (u64, bool) {
    if (_coll_recieved != 0) {
        return (_coll_recieved, true);
    } else {
        return (_requested_coll_withdrawn, false);
    }
}

fn internal_get_new_icr_from_trove_change(
    _coll: u64,
    _debt: u64,
    _coll_change: u64,
    _is_coll_increase: bool,
    _debt_change: u64,
    _is_debt_increase: bool,
    _price: u64,
) -> u64 {
    let new_position = internal_get_new_trove_amounts(_coll, _debt, _coll_change, _is_coll_increase, _debt_change, _is_debt_increase);

    let new_icr = fm_compute_cr(new_position.0, new_position.1, _price);

    return new_icr;
}

fn internal_get_new_nominal_icr_from_trove_change(
    _coll: u64,
    _debt: u64,
    _coll_change: u64,
    _is_coll_increase: bool,
    _debt_change: u64,
    _is_debt_increase: bool,
) -> u64 {
    let new_position = internal_get_new_trove_amounts(_coll, _debt, _coll_change, _is_coll_increase, _debt_change, _is_debt_increase);
    let new_icr = fm_compute_nominal_cr(new_position.0, new_position.1);

    return new_icr;
}

#[storage(read)]
fn internal_update_trove_from_adjustment(
    _borrower: Identity,
    _coll_change: u64,
    _is_coll_increase: bool,
    _debt_change: u64,
    _is_debt_increase: bool,
) -> (u64, u64) {
    let trove_manager = abi(TroveManager, storage.trove_manager_contract.value);
    let mut new_coll = 0;
    let mut new_debt = 0;

    if _is_coll_increase {
        new_coll = trove_manager.increase_trove_coll(_borrower, _coll_change);
    } else {
        new_coll = trove_manager.decrease_trove_coll(_borrower, _coll_change);
    }

    if _is_debt_increase {
        new_debt = trove_manager.increase_trove_debt(_borrower, _debt_change);
    } else {
        new_debt = trove_manager.decrease_trove_debt(_borrower, _debt_change);
    }

    return (new_coll, new_debt);
}

fn internal_get_new_trove_amounts(
    _coll: u64,
    _debt: u64,
    _coll_change: u64,
    _is_coll_increase: bool,
    _debt_change: u64,
    _is_debt_increase: bool,
) -> (u64, u64) {
    let new_coll = if _is_coll_increase {
        _coll + _coll_change
    } else {
        _coll - _coll_change
    };

    let new_debt = if _is_debt_increase {
        _debt + _debt_change
    } else {
        _debt - _debt_change
    };

    return (new_coll, new_debt);
}

#[storage(read)]
fn internal_active_pool_add_coll(_coll_change: u64) {
    let active_pool = abi(ActivePool, storage.active_pool_contract.value);

    active_pool.recieve {
        coins: _coll_change,
        asset_id: storage.asset_contract.value,
    }();
}

#[storage(read)]
fn internal_repay_usdf(usdf_amount: u64) {
    let active_pool = abi(ActivePool, storage.active_pool_contract.value);
    let usdf = abi(Token, storage.usdf_contract.value);

    transfer(usdf_amount, storage.usdf_contract, Identity::ContractId(storage.usdf_contract));
    usdf.burn_coins(usdf_amount);
    active_pool.decrease_usdf_debt(usdf_amount);
}

#[storage(read)]
fn internal_move_usdf_and_asset_from_adjustment(
    _borrower: Identity,
    _coll_change: u64,
    _is_coll_increase: bool,
    _usdf_change: u64,
    _is_debt_increase: bool,
    _net_debt_change: u64,
) {
    let active_pool = abi(ActivePool, storage.active_pool_contract.value);
    let usdf = abi(Token, storage.usdf_contract.value);

    if _coll_change > 0 {
        if _is_coll_increase {
            internal_active_pool_add_coll(_coll_change);
        } else {
            active_pool.send_asset(_borrower, _coll_change);
        }
    }

    if _usdf_change > 0 {
        if _is_debt_increase {
            withdraw_usdf(_borrower, _usdf_change, _net_debt_change);
        } else {
            internal_repay_usdf(_usdf_change);
        }
    }
}
