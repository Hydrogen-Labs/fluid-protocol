contract;
// This contract, BorrowOperations, manages borrowing operations for the USDF stablecoin system.
// It handles opening and adjusting troves, which are collateralized debt positions.
// The contract interacts with various other components of the system
//
// Key functionalities include:
// - Opening new troves
// - Adjusting existing troves (adding/removing collateral, borrowing/repaying USDF)
// - Closing troves
// - Managing multiple collateral asset types
// - Enforcing system parameters and stability conditions

mod data_structures;
mod events;

use standards::{src3::SRC3,};
use ::data_structures::{AssetContracts, LocalVariablesAdjustTrove, LocalVariablesOpenTrove};
use ::events::{AdjustTroveEvent, CloseTroveEvent, OpenTroveEvent};
use libraries::trove_manager_interface::data_structures::Status;
use libraries::active_pool_interface::ActivePool;
use libraries::token_interface::Token;
use libraries::trove_manager_interface::TroveManager;
use libraries::sorted_troves_interface::SortedTroves;
use libraries::fpt_staking_interface::FPTStaking;
use libraries::coll_surplus_pool_interface::CollSurplusPool;
use libraries::oracle_interface::Oracle;
use libraries::borrow_operations_interface::BorrowOperations;
use libraries::fluid_math::*;
use sway_libs::ownership::*;
use std::{
    asset::transfer,
    auth::msg_sender,
    call_frames::{
        msg_asset_id,
    },
    context::{
        msg_amount,
    },
    hash::*,
    logging::log,
};

configurable {
    /// Initializer identity
    INITIALIZER: Identity = Identity::Address(Address::zero()),
}

storage {
    asset_contracts: StorageMap<AssetId, AssetContracts> = StorageMap::<AssetId, AssetContracts> {},
    valid_asset_ids: StorageMap<AssetId, bool> = StorageMap::<AssetId, bool> {},
    usdf_contract: ContractId = ContractId::zero(),
    fpt_staking_contract: ContractId = ContractId::zero(),
    coll_surplus_pool_contract: ContractId = ContractId::zero(),
    active_pool_contract: ContractId = ContractId::zero(),
    protocol_manager_contract: ContractId = ContractId::zero(),
    sorted_troves_contract: ContractId = ContractId::zero(),
    usdf_asset_id: AssetId = AssetId::zero(),
    is_initialized: bool = false,
    is_paused: bool = false, // paused protocol still allows trove operations which do not increase trove debt
    is_asset_paused: StorageMap<AssetId, bool> = StorageMap::<AssetId, bool> {},
    pauser: Identity = Identity::Address(Address::zero()),
    lock_close_trove: bool = false,
    lock_internal_adjust_trove: bool = false,
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
        require(
            msg_sender()
                .unwrap() == INITIALIZER,
            "Borrow Operations: Caller is not initializer",
        );
        require(
            !storage
                .is_initialized
                .read(),
            "Borrow Operations: already initialized",
        );
        storage.usdf_contract.write(usdf_contract);
        storage.fpt_staking_contract.write(fpt_staking_contract);
        storage.protocol_manager_contract.write(protocol_manager);
        storage
            .coll_surplus_pool_contract
            .write(coll_surplus_pool_contract);
        storage.active_pool_contract.write(active_pool_contract);
        storage.sorted_troves_contract.write(sorted_troves_contract);
        storage
            .usdf_asset_id
            .write(AssetId::new(usdf_contract, SubId::zero()));
        storage.pauser.write(msg_sender().unwrap());
        initialize_ownership(msg_sender().unwrap());
        storage.is_initialized.write(true);
    }

    #[storage(read, write)]
    fn set_pauser(pauser: Identity) {
        only_owner();
        storage.pauser.write(pauser);
    }

    #[storage(read, write)]
    fn transfer_owner(new_owner: Identity) {
        only_owner();
        transfer_ownership(new_owner);
    }

    #[storage(read, write)]
    fn renounce_owner() {
        only_owner();
        renounce_ownership();
    }

    // --- Borrower Trove Operations ---
    // Open a new trove by borrowing USDF
    // Differences from Liquity:0% frontend fees, no recovery mode, no gas compensation
    #[storage(read), payable]
    fn open_trove(usdf_amount: u64, upper_hint: Identity, lower_hint: Identity) {
        require_is_not_paused();
        require_asset_is_not_paused(msg_asset_id());
        require_valid_asset_id();
        let asset_contract = msg_asset_id();
        let asset_contracts = storage.asset_contracts.get(asset_contract).read();
        let usdf_contract = storage.usdf_contract.read();
        let fpt_staking_contract = storage.fpt_staking_contract.read();
        let active_pool_contract = storage.active_pool_contract.read();
        let sorted_troves_contract = storage.sorted_troves_contract.read();
        let oracle = abi(Oracle, asset_contracts.oracle.bits());
        let trove_manager = abi(TroveManager, asset_contracts.trove_manager.bits());
        let sorted_troves = abi(SortedTroves, sorted_troves_contract.bits());
        let mut vars = LocalVariablesOpenTrove::new();
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
        internal_withdraw_usdf(
            sender,
            usdf_amount,
            vars.net_debt,
            active_pool_contract,
            usdf_contract,
            asset_contract,
        );
        log(OpenTroveEvent {
            user: sender,
            asset_id: asset_contract,
            collateral: msg_amount(),
            debt: vars.net_debt,
        });
    }
    // Add collateral to an existing trove
    #[storage(read, write), payable]
    fn add_coll(upper_hint: Identity, lower_hint: Identity) {
        require_valid_asset_id();
        internal_adjust_trove(
            msg_sender()
                .unwrap(),
            msg_amount(),
            0,
            0,
            false,
            upper_hint,
            lower_hint,
            msg_asset_id(),
        );
    }
    // Withdraw collateral from an existing trove
    #[storage(read, write)]
    fn withdraw_coll(
        amount: u64,
        upper_hint: Identity,
        lower_hint: Identity,
        asset_contract: AssetId,
    ) {
        internal_adjust_trove(
            msg_sender()
                .unwrap(),
            0,
            amount,
            0,
            false,
            upper_hint,
            lower_hint,
            asset_contract,
        );
    }
    // Withdraw USDF from an existing trove
    #[storage(read, write)]
    fn withdraw_usdf(
        amount: u64,
        upper_hint: Identity,
        lower_hint: Identity,
        asset_contract: AssetId,
    ) {
        internal_adjust_trove(
            msg_sender()
                .unwrap(),
            0,
            0,
            amount,
            true,
            upper_hint,
            lower_hint,
            asset_contract,
        );
    }
    // Repay USDF for an existing trove
    #[storage(read, write), payable]
    fn repay_usdf(
        upper_hint: Identity,
        lower_hint: Identity,
        asset_contract: AssetId,
    ) {
        require_valid_usdf_id(msg_asset_id());
        internal_adjust_trove(
            msg_sender()
                .unwrap(),
            0,
            0,
            msg_amount(),
            false,
            upper_hint,
            lower_hint,
            asset_contract,
        );
    }
    // Close an existing trove
    #[storage(read, write), payable]
    fn close_trove(asset_contract: AssetId) {
        require(
            storage
                .lock_close_trove
                .read() == false,
            "BorrowOperations: Close trove is locked",
        );
        storage.lock_close_trove.write(true);

        // Read all storage values at the beginning
        let asset_contracts_cache = storage.asset_contracts.get(asset_contract).read();
        let usdf_contract_cache = storage.usdf_contract.read();
        let active_pool_contract_cache = storage.active_pool_contract.read();
        let usdf_asset_id = storage.usdf_asset_id.read();

        let trove_manager = abi(TroveManager, asset_contracts_cache.trove_manager.bits());
        let active_pool = abi(ActivePool, active_pool_contract_cache.bits());
        let borrower = msg_sender().unwrap();

        require_trove_is_active(borrower, asset_contracts_cache.trove_manager);
        trove_manager.apply_pending_rewards(borrower);
        let coll = trove_manager.get_trove_coll(borrower);
        let debt = trove_manager.get_trove_debt(borrower);

        if debt > 0 {
            require_valid_usdf_id(msg_asset_id());
            require(
                debt <= msg_amount(),
                "Borrow Operations: cannot close trove with insufficient usdf balance",
            );
        }

        trove_manager.remove_stake(borrower);
        trove_manager.close_trove(borrower);

        internal_repay_usdf(
            debt,
            active_pool_contract_cache,
            usdf_contract_cache,
            asset_contract,
        );
        active_pool.send_asset(borrower, coll, asset_contract);
        if (debt < msg_amount()) {
            require_valid_usdf_id(msg_asset_id());
            let excess_usdf_returned = msg_amount() - debt;
            transfer(borrower, usdf_asset_id, excess_usdf_returned);
        }

        log(CloseTroveEvent {
            user: borrower,
            asset_id: asset_contract,
            collateral: coll,
            debt: debt,
        });
        storage.lock_close_trove.write(false);
    }
    // Claim collateral from liquidations
    #[storage(read)]
    fn claim_collateral(asset: AssetId) {
        let coll_surplus = abi(CollSurplusPool, storage.coll_surplus_pool_contract.read().bits());
        coll_surplus.claim_coll(msg_sender().unwrap(), asset);
    }
    #[storage(read)]
    fn get_usdf_asset_id() -> AssetId {
        return storage.usdf_asset_id.read();
    }
    #[storage(read, write)]
    fn set_pause_status(is_paused: bool) {
        require_is_pauser();
        storage.is_paused.write(is_paused);
    }

    #[storage(read, write)]
    fn set_asset_pause_status(asset: AssetId, is_paused: bool) {
        require_is_pauser();
        storage.is_asset_paused.insert(asset, is_paused);
    }

    #[storage(read)]
    fn get_asset_pause_status(asset: AssetId) -> bool {
        return storage.is_asset_paused.get(asset).try_read().unwrap_or(false);
    }

    #[storage(read)]
    fn get_pauser() -> Identity {
        return storage.pauser.read();
    }

    #[storage(read)]
    fn get_is_paused() -> bool {
        return storage.is_paused.read();
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
        storage
            .asset_contracts
            .insert(asset_contract, asset_contracts);
    }
}

// --- Internal Functions ---
// Note: flat borrowing fee
fn internal_trigger_borrowing_fee(
    usdf_amount: u64,
    usdf_contract: ContractId,
    fpt_staking_contract: ContractId,
) -> u64 {
    let usdf = abi(SRC3, usdf_contract.bits());
    let fpt_staking = abi(FPTStaking, fpt_staking_contract.bits());
    let usdf_fee = fm_compute_borrow_fee(usdf_amount);

    //increase fpt staking rewards
    fpt_staking.increase_f_usdf(usdf_fee);
    // Mint usdf to fpt staking contract
    usdf.mint(
        Identity::ContractId(fpt_staking_contract),
        Some(SubId::zero()),
        usdf_fee,
    );

    return usdf_fee
}
// Note: no frontend fees
#[storage(read, write)]
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
    require(
        storage
            .lock_internal_adjust_trove
            .read() == false,
        "BorrowOperations: Internal adjust trove is locked",
    );
    storage.lock_internal_adjust_trove.write(true);

    let asset_contracts_cache = storage.asset_contracts.get(asset).read();
    let usdf_contract_cache = storage.usdf_contract.read();
    let fpt_staking_contract_cache = storage.fpt_staking_contract.read();
    let active_pool_contract_cache = storage.active_pool_contract.read();
    let sorted_troves_contract_cache = storage.sorted_troves_contract.read();
    let oracle = abi(Oracle, asset_contracts_cache.oracle.bits());
    let trove_manager = abi(TroveManager, asset_contracts_cache.trove_manager.bits());
    let sorted_troves = abi(SortedTroves, sorted_troves_contract_cache.bits());
    let price = oracle.get_price();
    let mut vars = LocalVariablesAdjustTrove::new();
    if is_debt_increase {
        require_is_not_paused();
        require_asset_is_not_paused(asset);
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
        vars.usdf_fee = internal_trigger_borrowing_fee(
            vars.net_debt_change,
            usdf_contract_cache,
            fpt_staking_contract_cache,
        );
        vars.net_debt_change = vars.net_debt_change + vars.usdf_fee;
    }
    vars.debt = trove_manager.get_trove_debt(borrower);
    vars.coll = trove_manager.get_trove_coll(borrower);
    vars.old_icr = fm_compute_cr(vars.coll, vars.debt, price);
    vars.new_icr = internal_get_new_icr_from_trove_change(
        vars.coll,
        vars.debt,
        vars.coll_change,
        vars.is_coll_increase,
        vars.net_debt_change,
        is_debt_increase,
        price,
    );
    require(
        coll_withdrawal <= vars.coll,
        "Cannot withdraw more than the Trove's collateral",
    );
    require_at_least_mcr(vars.new_icr);

    if !is_debt_increase && usdf_change > 0 {
        require_at_least_min_net_debt(vars.debt - vars.net_debt_change);
    }

    let new_position_res = internal_update_trove_from_adjustment(
        borrower,
        vars.coll_change,
        vars.is_coll_increase,
        vars.net_debt_change,
        is_debt_increase,
        asset_contracts_cache
            .trove_manager,
    );
    let _ = trove_manager.update_stake_and_total_stakes(borrower);
    let new_nicr = internal_get_new_nominal_icr_from_trove_change(
        vars.coll,
        vars.debt,
        vars.coll_change,
        vars.is_coll_increase,
        vars.net_debt_change,
        is_debt_increase,
    );
    sorted_troves.re_insert(borrower, new_nicr, upper_hint, lower_hint, asset);
    internal_move_usdf_and_asset_from_adjustment(
        borrower,
        vars.coll_change,
        vars.is_coll_increase,
        usdf_change,
        is_debt_increase,
        vars.net_debt_change,
        asset,
        active_pool_contract_cache,
        usdf_contract_cache,
    );

    log(AdjustTroveEvent {
        user: borrower,
        asset_id: asset,
        collateral_change: vars.coll_change,
        debt_change: vars.net_debt_change,
        is_collateral_increase: vars.is_coll_increase,
        is_debt_increase: is_debt_increase,
        total_collateral: new_position_res.0,
        total_debt: new_position_res.1,
    });
    storage.lock_internal_adjust_trove.write(false);
}

#[storage(read)]
fn require_is_protocol_manager() {
    let protocol_manager = Identity::ContractId(storage.protocol_manager_contract.read());
    require(
        msg_sender()
            .unwrap() == protocol_manager,
        "Borrow Operations: Caller is not the protocol manager",
    );
}
#[storage(read)]
fn require_is_pauser() {
    require(
        msg_sender()
            .unwrap() == storage
            .pauser
            .read(),
        "Borrow Operations: Caller is not the pauser",
    );
}
#[storage(read)]
fn require_is_not_paused() {
    require(
        !storage
            .is_paused
            .read(),
        "Borrow Operations: Contract is paused",
    );
}

#[storage(read)]
fn require_asset_is_not_paused(asset: AssetId) {
    require(
        !storage
            .is_asset_paused
            .get(asset)
            .try_read()
            .unwrap_or(false),
        "Borrow Operations: Asset is paused",
    );
}

fn require_trove_is_not_active(borrower: Identity, trove_manager: ContractId) {
    let trove_manager = abi(TroveManager, trove_manager.bits());
    let status = trove_manager.get_trove_status(borrower);
    require(
        status != Status::Active,
        "Borrow Operations: User already has an active Trove",
    );
}
fn require_trove_is_active(borrower: Identity, trove_manage_contract: ContractId) {
    let trove_manager = abi(TroveManager, trove_manage_contract.bits());
    let status = trove_manager.get_trove_status(borrower);
    require(
        status == Status::Active,
        "Borrow Operations: User does not have an active Trove",
    );
}
fn require_non_zero_adjustment(asset_amount: u64, coll_withdrawl: u64, usdf_change: u64) {
    require(
        asset_amount > 0 || coll_withdrawl > 0 || usdf_change > 0,
        "Borrow Operations: coll withdrawal and debt change must be greater than 0",
    );
}
fn require_at_least_min_net_debt(_net_debt: u64) {
    require(
        _net_debt >= MIN_NET_DEBT,
        "Borrow Operations: net debt must be greater than 0",
    );
}
fn require_non_zero_debt_change(debt_change: u64) {
    require(
        debt_change > 0,
        "Borrow Operations: debt change must be greater than 0",
    );
}
fn require_at_least_mcr(icr: u64) {
    require(
        icr >= MCR,
        "Borrow Operations: Minimum collateral ratio not met",
    );
}
fn require_singular_coll_change(coll_added_amount: u64, coll_withdrawl: u64) {
    require(
        coll_withdrawl == 0 || 0 == coll_added_amount,
        "Borrow Operations: collateral change must be 0 or equal to the amount sent",
    );
}
#[storage(read)]
fn require_valid_asset_id() {
    require(
        storage
            .valid_asset_ids
            .get(msg_asset_id())
            .read(),
        "Borrow Operations: Invalid collateral asset being transfered",
    );
}
#[storage(read)]
fn require_valid_usdf_id(recieved_asset: AssetId) {
    require(
        recieved_asset == storage
            .usdf_asset_id
            .read(),
        "Borrow Operations: Invalid USDF asset being transfered",
    );
}
fn internal_withdraw_usdf(
    recipient: Identity,
    amount: u64,
    net_debt_increase: u64,
    active_pool_contract: ContractId,
    usdf_contract: ContractId,
    asset_contract: AssetId,
) {
    let active_pool = abi(ActivePool, active_pool_contract.bits());
    let usdf = abi(SRC3, usdf_contract.bits());
    active_pool.increase_usdf_debt(net_debt_increase, asset_contract);
    usdf.mint(recipient, Some(SubId::zero()), amount);
}
fn internal_get_coll_change(coll_recieved: u64, requested_coll_withdrawn: u64) -> (u64, bool) {
    if (coll_recieved != 0) {
        return (coll_recieved, true);
    } else {
        return (requested_coll_withdrawn, false);
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
    let new_position = internal_get_new_trove_amounts(
        coll,
        debt,
        coll_change,
        is_coll_increase,
        debt_change,
        is_debt_increase,
    );
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
    let new_position = internal_get_new_trove_amounts(
        coll,
        debt,
        coll_change,
        is_coll_increase,
        debt_change,
        is_debt_increase,
    );
    let new_icr = fm_compute_nominal_cr(new_position.0, new_position.1);
    return new_icr;
}
fn internal_update_trove_from_adjustment(
    borrower: Identity,
    coll_change: u64,
    is_coll_increase: bool,
    debt_change: u64,
    is_debt_increase: bool,
    trove_manager: ContractId,
) -> (u64, u64) {
    let trove_manager = abi(TroveManager, trove_manager.bits());
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
fn internal_active_pool_add_coll(coll_change: u64, asset: AssetId, active_pool: ContractId) {
    let active_pool = abi(ActivePool, active_pool.bits());
    active_pool
        .recieve {
            coins: coll_change,
            asset_id: asset.bits(),
        }();
}
#[storage(read)]
fn internal_repay_usdf(
    usdf_amount: u64,
    active_pool_contract: ContractId,
    usdf_contract: ContractId,
    asset_contract: AssetId,
) {
    let active_pool = abi(ActivePool, active_pool_contract.bits());
    let usdf = abi(SRC3, usdf_contract.bits());
    usdf
        .burn {
            coins: usdf_amount,
            asset_id: storage.usdf_asset_id.read().bits(),
        }(SubId::zero(), usdf_amount);

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
    let active_pool = abi(ActivePool, active_pool_contract.bits());
    if coll_change > 0 {
        if is_coll_increase {
            internal_active_pool_add_coll(coll_change, asset, active_pool_contract);
        } else {
            active_pool.send_asset(borrower, coll_change, asset);
        }
    }
    if usdf_change > 0 {
        if is_debt_increase {
            internal_withdraw_usdf(
                borrower,
                usdf_change,
                net_debt_change,
                active_pool_contract,
                usdf_contract,
                asset,
            );
        } else {
            internal_repay_usdf(usdf_change, active_pool_contract, usdf_contract, asset);
        }
    }
}
