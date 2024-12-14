contract;

// This contract, StabilityPool, manages the Stability Pool in the Fluid Protocol.
// The Stability Pool holds USDF deposits from users and plays a crucial role in the liquidation process.
//
// Key functionalities include:
// - Managing user deposits and withdrawals of USDF
// - Handling the offset of debt during trove liquidations
// - Distributing gains (collateral assets and FPT tokens) to depositors
// - Maintaining internal accounting of deposits, gains, and scale factors
// - Interfacing with other core contracts like TroveManager, ActivePool, and CommunityIssuance
//
// The contract uses a system of epochs, scales, and snapshots to accurately
// track and distribute gains to depositors over time, even as the total deposits fluctuate.
// Solidity reference: https://github.com/liquity/dev/blob/main/packages/contracts/contracts/StabilityPool.sol

mod data_structures;
mod events;
use ::data_structures::{AssetContracts, Snapshots};
use ::events::{
    ProvideToStabilityPoolEvent,
    StabilityPoolLiquidationEvent,
    WithdrawFromStabilityPoolEvent,
};

use standards::src3::SRC3;
use libraries::trove_manager_interface::data_structures::Status;
use libraries::stability_pool_interface::StabilityPool;
use libraries::usdf_token_interface::USDFToken;
use libraries::oracle_interface::Oracle;
use libraries::active_pool_interface::ActivePool;
use libraries::trove_manager_interface::TroveManager;
use libraries::borrow_operations_interface::BorrowOperations;
use libraries::community_issuance_interface::CommunityIssuance;
use libraries::sorted_troves_interface::SortedTroves;
use libraries::fluid_math::numbers::*;
use libraries::fluid_math::{DECIMAL_PRECISION, fm_min, MCR, null_contract, null_identity_address,};
use std::{
    asset::transfer,
    call_frames::{
        msg_asset_id,
    },
    context::{
        msg_amount,
    },
    hash::Hash,
    storage::storage_vec::*,
    u128::U128,
};
const SCALE_FACTOR: u64 = 10_000;
configurable {
    /// Initializer identity
    INITIALIZER: Identity = Identity::Address(Address::zero()),
}

storage {
    asset_contracts: StorageMap<AssetId, AssetContracts> = StorageMap::<AssetId, AssetContracts> {},
    active_pool_contract: ContractId = ContractId::zero(),
    protocol_manager_address: ContractId = ContractId::zero(),
    usdf_contract: ContractId = ContractId::zero(),
    usdf_asset_id: AssetId = AssetId::zero(),
    community_issuance_contract: ContractId = ContractId::zero(),
    sorted_troves_contract: ContractId = ContractId::zero(),
    // List of assets tracked by the Stability Pool
    valid_assets: StorageVec<AssetId> = StorageVec {},
    // Asset amounts held by the Stability Pool to be claimed
    asset: StorageMap<AssetId, u64> = StorageMap::<AssetId, u64> {},
    // Total amount of USDF held by the Stability Pool
    total_usdf_deposits: u64 = 0,
    // Amount of USDF deposited by each user
    deposits: StorageMap<Identity, u64> = StorageMap::<Identity, u64> {},
    // Starting S for each asset when deposited by each user
    deposit_snapshot_s_per_asset: StorageMap<(Identity, AssetId), U128> = StorageMap::<(Identity, AssetId), U128> {},
    // Snapshot of P, G, epoch and scale when each deposit was made
    deposit_snapshots: StorageMap<Identity, Snapshots> = StorageMap::<Identity, Snapshots> {},
    // Each time the scale of P shifts by SCALE_FACTOR, the scale is incremented by 1
    current_scale: u64 = 0,
    // With each offset that fully empties the Pool, the epoch is incremented by 1
    current_epoch: u64 = 0,
    /* Asset Gain sum 'S(asset)': During its lifetime, each deposit d_t earns an Asset gain of ( d_t * [S(asset) - S_t(asset)] )/P_t, 
    * where S_t(asset) is the depositor's snapshot of S(asset) taken at the time t when the deposit was made.
    *
    * The 'S(asset)' sums are stored in a nested mapping (epoch => scale => asset => sum):
    *
    * - The inner mapping records the sum S(asset) at different scales
    * - The outer mapping records the (scale => sum) mappings, for different epochs.
    */
    epoch_to_scale_to_sum: StorageMap<(u64, u64, AssetId), U128> = StorageMap::<(u64, u64, AssetId), U128> {},
    /*
    * Similarly, the sum 'G' is used to calculate FPT gains. During it's lifetime, each deposit d_t earns a FPT gain of
    *  ( d_t * [G - G_t] )/P_t, where G_t is the depositor's snapshot of G taken at time t when  the deposit was made.
    *
    *  FPT reward events occur are triggered by depositor operations (new deposit, topup, withdrawal), and liquidations.
    *  In each case, the FPT reward is issued (i.e. G is updated), before other state changes are made.
    */
    epoch_to_scale_to_gain: StorageMap<(u64, u64), U128> = StorageMap::<(u64, u64), U128> {},
    /*   * --- EPOCHS ---
    *
    * Whenever a liquidation fully empties the Stability Pool, all deposits should become 0. However, setting P to 0 would make P be 0
    * forever, and break all future reward calculations.
    *
    * So, every time the Stability Pool is emptied by a liquidation, we reset P = 1 and currentScale = 0, and increment the currentEpoch by 1.
    */
    p: U128 = U128::from(DECIMAL_PRECISION),
    last_fpt_error: U128 = U128::zero(),
    last_asset_error_offset: StorageMap<AssetId, U128> = StorageMap::<AssetId, U128> {},
    last_usdf_error_offset: U128 = U128::zero(),
    is_initialized: bool = false,
    // Locks to prevent reentrancy for functions not using Checks-Effects-Interactions pattern
    lock_provide_to_stability_pool: bool = false,
    lock_withdraw_from_stability_pool: bool = false,
    lock_offset: bool = false,
}
impl StabilityPool for Contract {
    #[storage(read, write)]
    fn initialize(
        usdf_contract: ContractId,
        community_issuance_contract: ContractId,
        protocol_manager: ContractId,
        active_pool_contract: ContractId,
        sorted_troves_contract: ContractId,
    ) {
        require(
            msg_sender()
                .unwrap() == INITIALIZER,
            "StabilityPool: Caller is not initializer",
        );
        require(
            storage
                .is_initialized
                .read() == false,
            "StabilityPool: Contract is already initialized",
        );
        storage.usdf_contract.write(usdf_contract);
        storage
            .community_issuance_contract
            .write(community_issuance_contract);
        storage.protocol_manager_address.write(protocol_manager);
        storage.active_pool_contract.write(active_pool_contract);
        storage.sorted_troves_contract.write(sorted_troves_contract);
        storage.is_initialized.write(true);
        storage
            .usdf_asset_id
            .write(AssetId::new(usdf_contract, SubId::zero()));
    }
    #[storage(read, write)]
    fn add_asset(
        trove_manager_contract: ContractId,
        asset_contract: AssetId,
        oracle_contract: ContractId,
    ) {
        require_is_protocol_manager();
        storage.valid_assets.push(asset_contract);
        storage
            .last_asset_error_offset
            .insert(asset_contract, U128::zero());
        storage
            .asset_contracts
            .insert(
                asset_contract,
                AssetContracts {
                    trove_manager: trove_manager_contract,
                    oracle: oracle_contract,
                },
            );
    }
    /*
    * - Triggers a FPT issuance, based on time passed since the last issuance. The FPT issuance is shared between *all* depositors
    * - Sends depositor's accumulated gains (FPT, Asset1, Asset2...) to depositor
    * - Sends the tagged front end's accumulated FPT gains to the tagged front end
    * - Increases deposit stake, and takes new snapshots for each.
    */
    #[storage(read, write), payable]
    fn provide_to_stability_pool() {
        require(
            storage
                .lock_provide_to_stability_pool
                .read() == false,
            "StabilityPool: Contract is locked",
        );
        storage.lock_provide_to_stability_pool.write(true);
        require_usdf_is_valid_and_non_zero();
        let initial_deposit = storage.deposits.get(msg_sender().unwrap()).try_read().unwrap_or(0);
        internal_trigger_fpt_issuance();
        let compounded_usdf_deposit = internal_get_compounded_usdf_deposit(msg_sender().unwrap());
        internal_pay_out_asset_gains(msg_sender().unwrap()); // pay out asset gains
        internal_pay_out_fpt_gains(msg_sender().unwrap());
        let new_position = compounded_usdf_deposit + msg_amount();
        internal_update_deposits_and_snapshots(msg_sender().unwrap(), new_position);
        storage
            .total_usdf_deposits
            .write(storage.total_usdf_deposits.read() + msg_amount());
        log(ProvideToStabilityPoolEvent {
            user: msg_sender().unwrap(),
            amount_to_deposit: msg_amount(),
            initial_amount: initial_deposit,
            compounded_amount: compounded_usdf_deposit,
        });
        storage.lock_provide_to_stability_pool.write(false);
    }
    /*
    * - Triggers a FPT issuance, based on time passed since the last issuance. The FPT issuance is shared between *all* depositors
    * - Sends all depositor's accumulated gains (FPT, Asset1, Asset2...) to depositor
    * - Decreases deposit stake, and takes new snapshots for each.
    *
    * If amount > userDeposit, the user withdraws all of their compounded deposit.
    */
    #[storage(read, write)]
    fn withdraw_from_stability_pool(amount: u64) {
        require(
            storage
                .lock_withdraw_from_stability_pool
                .read() == false,
            "StabilityPool: Withdraw is locked",
        );
        storage.lock_withdraw_from_stability_pool.write(true);
        require_no_undercollateralized_troves();
        let initial_deposit = storage.deposits.get(msg_sender().unwrap()).try_read().unwrap_or(0);
        require_user_has_initial_deposit(initial_deposit);
        internal_trigger_fpt_issuance();
        let compounded_usdf_deposit = internal_get_compounded_usdf_deposit(msg_sender().unwrap());
        let usdf_to_withdraw = fm_min(amount, compounded_usdf_deposit);
        let new_position = compounded_usdf_deposit - usdf_to_withdraw;
        internal_pay_out_asset_gains(msg_sender().unwrap()); // pay out asset gains
        internal_pay_out_fpt_gains(msg_sender().unwrap()); // pay out FPT
        internal_update_deposits_and_snapshots(msg_sender().unwrap(), new_position);
        send_usdf_to_depositor(msg_sender().unwrap(), usdf_to_withdraw);
        log(WithdrawFromStabilityPoolEvent {
            user: msg_sender().unwrap(),
            amount_to_withdraw: usdf_to_withdraw,
            initial_amount: initial_deposit,
            compounded_amount: compounded_usdf_deposit,
        });
        storage.lock_withdraw_from_stability_pool.write(false);
    }
    /*
    * Cancels out the specified debt against the USDF contained in the Stability Pool (as far as possible)
    * and transfers the Trove's asset collateral from ActivePool to StabilityPool.
    * Only called by liquidation functions in the TroveManager.
    */
    #[storage(read, write)]
    fn offset(
        debt_to_offset: u64,
        coll_to_offset: u64,
        asset_contract: AssetId,
    ) {
        require(
            storage
                .lock_offset
                .read() == false,
            "StabilityPool: Offset is locked",
        );
        storage.lock_offset.write(true);
        require_caller_is_trove_manager();
        let total_usdf = storage.total_usdf_deposits.read();
        if total_usdf == 0 || debt_to_offset == 0 {
            storage.lock_offset.write(false);
            return;
        }
        internal_trigger_fpt_issuance();
        let per_unit_staked_changes = compute_rewards_per_unit_staked(coll_to_offset, debt_to_offset, total_usdf, asset_contract);
        update_reward_sum_and_product(
            per_unit_staked_changes.0,
            per_unit_staked_changes.1,
            asset_contract,
        );
        internal_move_offset_coll_and_debt(coll_to_offset, debt_to_offset, asset_contract);
        log(StabilityPoolLiquidationEvent {
            asset_id: asset_contract,
            debt_to_offset: debt_to_offset,
            collateral_to_offset: coll_to_offset,
        });
        storage.lock_offset.write(false);
    }
    #[storage(read)]
    fn get_asset(asset_contract: AssetId) -> u64 {
        return storage.asset.get(asset_contract).try_read().unwrap_or(0);
    }
    #[storage(read)]
    fn get_total_usdf_deposits() -> u64 {
        return storage.total_usdf_deposits.try_read().unwrap_or(0);
    }
    #[storage(read)]
    fn get_depositor_asset_gain(depositor: Identity, asset_contract: AssetId) -> u64 {
        return internal_get_depositor_asset_gain(depositor, asset_contract);
    }
    #[storage(read)]
    fn get_compounded_usdf_deposit(depositor: Identity) -> u64 {
        return internal_get_compounded_usdf_deposit(depositor);
    }
    #[storage(read)]
    fn get_depositor_fpt_gain(depositor: Identity) -> u64 {
        return internal_get_depositor_fpt_gain(depositor);
    }
}
// --- Internal functions ---
#[storage(read, write)]
fn internal_pay_out_asset_gains(depositor: Identity) {
    let mut i = 0;
    while i < storage.valid_assets.len() {
        let asset_contract = storage.valid_assets.get(i).unwrap().read();
        let asset_gain = internal_get_depositor_asset_gain(depositor, asset_contract);
        send_asset_gain_to_depositor(depositor, asset_gain, asset_contract);
        i += 1;
    }
}
#[storage(read, write)]
fn internal_trigger_fpt_issuance() {
    let community_issuance_contract = abi(CommunityIssuance, storage.community_issuance_contract.read().bits());
    let fpt_issuance = community_issuance_contract.issue_fpt();
    internal_update_g(fpt_issuance);
}
#[storage(read, write)]
fn internal_update_g(fpt_issuance: u64) {
    if (storage.total_usdf_deposits.read() == 0
        || fpt_issuance == 0)
    {
        return;
    }
    let fpt_per_unit_staked = internal_compute_fpt_per_unit_staked(fpt_issuance, storage.total_usdf_deposits.read());
    let marginal_fpt_gain = U128::from(fpt_per_unit_staked) * storage.p.read();
    let current_epoch = storage.current_epoch.read();
    let current_scale = storage.current_scale.read();
    let new_epoch_to_scale_to_gain = storage.epoch_to_scale_to_gain.get((current_epoch, current_scale)).try_read().unwrap_or(U128::zero()) + marginal_fpt_gain;
    storage
        .epoch_to_scale_to_gain
        .insert((current_epoch, current_scale), new_epoch_to_scale_to_gain);
}
#[storage(read, write)]
fn internal_compute_fpt_per_unit_staked(fpt_issuance: u64, total_usdf_deposits: u64) -> u64 {
    let fpt_numerator = U128::from(fpt_issuance) * U128::from(DECIMAL_PRECISION) + storage.last_fpt_error.read();
    let fpt_per_unit_staked = fpt_numerator / U128::from(total_usdf_deposits);
    storage
        .last_fpt_error
        .write(fpt_numerator - (fpt_per_unit_staked * U128::from(total_usdf_deposits)));
    fpt_per_unit_staked.as_u64().unwrap()
}
#[storage(read)]
fn internal_pay_out_fpt_gains(depositor: Identity) {
    let depositor_fpt_gain = internal_get_depositor_fpt_gain(depositor);
    if (depositor_fpt_gain > 0) {
        let community_issuance_contract = abi(CommunityIssuance, storage.community_issuance_contract.read().bits());
        community_issuance_contract.send_fpt(depositor, depositor_fpt_gain);
    }
}
#[storage(read)]
fn internal_get_depositor_fpt_gain(depositor: Identity) -> u64 {
    let initial_deposit = storage.deposits.get(depositor).try_read().unwrap_or(0);
    if (initial_deposit == 0) {
        return 0;
    }
    let snapshots = storage.deposit_snapshots.get(depositor).try_read().unwrap_or(Snapshots::default());
    let fpt_gain = internal_get_fpt_gain_from_snapshots(initial_deposit, snapshots);
    fpt_gain
}
#[storage(read)]
fn internal_get_fpt_gain_from_snapshots(initial_stake: u64, snapshots: Snapshots) -> u64 {
    let epoch_snapshot = snapshots.epoch;
    let scale_snapshot = snapshots.scale;
    let g_snapshot = snapshots.G;
    let p_snapshot = snapshots.P;
    let first_portion = storage.epoch_to_scale_to_gain.get((epoch_snapshot, scale_snapshot)).try_read().unwrap_or(U128::zero()) - g_snapshot;
    let second_portion = storage.epoch_to_scale_to_gain.get((epoch_snapshot, scale_snapshot + 1)).try_read().unwrap_or(U128::zero()) / U128::from(SCALE_FACTOR);
    let gain = (U128::from(initial_stake) * (first_portion + second_portion)) / p_snapshot / U128::from(DECIMAL_PRECISION);
    return gain.as_u64().unwrap();
}
#[storage(read)]
fn require_is_protocol_manager() {
    let protocol_manager = Identity::ContractId(storage.protocol_manager_address.read());
    require(
        msg_sender()
            .unwrap() == protocol_manager,
        "StabilityPool: Caller is not the protocol manager",
    );
}
#[storage(read)]
fn require_usdf_is_valid_and_non_zero() {
    require(
        storage
            .usdf_asset_id
            .read() == msg_asset_id(),
        "StabilityPool: USDF address is invalid",
    );
    require(
        msg_amount() > 0,
        "StabilityPool: USDF amount must be greater than 0",
    );
}
#[storage(read)]
fn require_user_has_trove(address: Identity, trove_manager_contract: ContractId) {
    let trove_manager = abi(TroveManager, trove_manager_contract.bits());
    let status = trove_manager.get_trove_status(address);
    require(
        status == Status::Active,
        "StabilityPool: User does not have an active trove",
    );
}
// --- Reward calculator functions for depositor and front end ---
#[storage(read)]
fn internal_get_depositor_asset_gain(depositor: Identity, asset: AssetId) -> u64 {
    let initial_deposit = storage.deposits.get(depositor).try_read().unwrap_or(0);
    if initial_deposit == 0 {
        return 0;
    }
    let s_snapshot = storage.deposit_snapshot_s_per_asset.get((depositor, asset)).try_read().unwrap_or(U128::zero());
    let mut snapshots = storage.deposit_snapshots.get(depositor).try_read().unwrap_or(Snapshots::default());
    return internal_get_asset_gain_from_snapshots(initial_deposit, snapshots, s_snapshot, asset);
}
#[storage(read)]
fn internal_get_asset_gain_from_snapshots(
    initial_deposit: u64,
    snapshots: Snapshots,
    s_snapshot: U128,
    asset: AssetId,
) -> u64 {
    let epoch_snapshot = snapshots.epoch;
    let scale_snapshot = snapshots.scale;
    let p_snapshot = snapshots.P;
    let first_portion = storage.epoch_to_scale_to_sum.get((epoch_snapshot, scale_snapshot, asset)).try_read().unwrap_or(U128::zero()) - s_snapshot;
    let second_portion = storage.epoch_to_scale_to_sum.get((epoch_snapshot, scale_snapshot + 1, asset)).try_read().unwrap_or(U128::zero()) / U128::from(SCALE_FACTOR);
    let gain = (U128::from(initial_deposit) * (first_portion + second_portion)) / p_snapshot / U128::from(DECIMAL_PRECISION);
    return gain.as_u64().unwrap();
}
#[storage(read)]
fn internal_get_compounded_usdf_deposit(depositor: Identity) -> u64 {
    let initial_deposit = storage.deposits.get(depositor).try_read().unwrap_or(0);
    if initial_deposit == 0 {
        return 0;
    }
    let mut snapshots = storage.deposit_snapshots.get(depositor).read();
    return get_compounded_stake_from_snapshots(initial_deposit, snapshots)
}
#[storage(read)]
fn get_compounded_stake_from_snapshots(initial_stake: u64, snapshots: Snapshots) -> u64 {
    let epoch_snapshot = snapshots.epoch;
    let scale_snapshot = snapshots.scale;
    let p_snapshot = snapshots.P;
    if (epoch_snapshot < storage.current_epoch.read()) {
        return 0;
    }
    let mut compounded_stake: U128 = U128::zero();
    let scale_diff = storage.current_scale.read() - scale_snapshot;
    if (scale_diff == 0) {
        compounded_stake = U128::from(initial_stake) * storage.p.read() / p_snapshot;
    } else if (scale_diff == 1) {
        compounded_stake = U128::from(initial_stake) * storage.p.read() / p_snapshot / U128::from(SCALE_FACTOR);
    } else {
        compounded_stake = U128::zero();
    }
    if (compounded_stake < U128::from(initial_stake) / U128::from(DECIMAL_PRECISION))
    {
        return 0;
    }
    return compounded_stake.as_u64().unwrap();
}
#[storage(read, write)]
fn internal_decrease_usdf(total_usdf_to_decrease: u64) {
    storage
        .total_usdf_deposits
        .write(storage.total_usdf_deposits.read() - total_usdf_to_decrease);
}
#[storage(read, write)]
fn internal_increase_asset(total_asset_to_increase: u64, asset_contract: AssetId) {
    let mut asset_amount = storage.asset.get(asset_contract).try_read().unwrap_or(0);
    asset_amount += total_asset_to_increase;
    storage.asset.insert(asset_contract, asset_amount);
}
#[storage(read, write)]
fn internal_update_deposits_and_snapshots(depositor: Identity, amount: u64) {
    storage.deposits.insert(depositor, amount);
    if (amount == 0) {
        let _ = storage.deposit_snapshots.remove(depositor);
    }
    let current_epoch = storage.current_epoch.read();
    let current_scale = storage.current_scale.read();
    let current_p = storage.p.read();
    let current_g = storage.epoch_to_scale_to_gain.get((current_epoch, current_scale)).try_read().unwrap_or(U128::zero());
    let snapshots = Snapshots {
        epoch: current_epoch,
        scale: current_scale,
        P: current_p,
        G: current_g,
    };
    let mut i = 0;
    while i < storage.valid_assets.len() {
        let asset = storage.valid_assets.get(i).unwrap().read();
        let current_s: U128 = storage.epoch_to_scale_to_sum.get((current_epoch, current_scale, asset)).try_read().unwrap_or(U128::zero());
        storage
            .deposit_snapshot_s_per_asset
            .insert((depositor, asset), current_s);
        i += 1;
    }
    storage.deposit_snapshots.insert(depositor, snapshots);
}
#[storage(read, write), payable]
fn send_asset_gain_to_depositor(depositor: Identity, gain: u64, asset_contract: AssetId) {
    if (gain == 0) {
        return;
    }
    let mut asset_amount = storage.asset.get(asset_contract).read();
    asset_amount -= gain;
    storage.asset.insert(asset_contract, asset_amount);
    transfer(depositor, asset_contract, gain);
}
#[storage(read, write)]
fn send_usdf_to_depositor(depositor: Identity, amount: u64) {
    if (amount == 0) {
        return;
    }
    storage
        .total_usdf_deposits
        .write(storage.total_usdf_deposits.read() - amount);
    let usdf_asset_id = storage.usdf_asset_id.read();
    transfer(depositor, usdf_asset_id, amount);
}
#[storage(read)]
fn require_caller_is_trove_manager() {
    let mut i = 0;
    while i < storage.valid_assets.len() {
        let asset = storage.valid_assets.get(i).unwrap().read();
        let trove_manager_contract = Identity::ContractId(storage.asset_contracts.get(asset).read().trove_manager);
        if (msg_sender().unwrap() == trove_manager_contract) {
            return;
        }
        i += 1;
    }
    require(false, "StabilityPool: Caller is not a trove manager");
}
#[storage(read)]
fn require_no_undercollateralized_troves() {
    let sorted_troves = abi(SortedTroves, storage.sorted_troves_contract.read().into());
    let mut i = 0;
    while i < storage.valid_assets.len() {
        let asset = storage.valid_assets.get(i).unwrap().read();
        let asset_contracts = storage.asset_contracts.get(asset).read();
        let trove_manager = abi(TroveManager, asset_contracts.trove_manager.into());
        let oracle = abi(Oracle, asset_contracts.oracle.into());
        let price = oracle.get_price();
        let last = sorted_troves.get_last(asset);
        require(
            last == Identity::Address(Address::zero()) || trove_manager
                .get_current_icr(last, price) > MCR,
            "StabilityPool: There are undercollateralized troves",
        );
        i += 1;
    }
}
fn require_user_has_initial_deposit(deposit: u64) {
    require(deposit > 0, "StabilityPool: User has no initial deposit");
}
#[storage(read, write)]
fn compute_rewards_per_unit_staked(
    coll_to_add: u64,
    debt_to_offset: u64,
    total_usdf_deposits: u64,
    asset_contract: AssetId,
) -> (U128, U128) {
    let asset_numerator: U128 = U128::from(coll_to_add) * U128::from(DECIMAL_PRECISION) + storage.last_asset_error_offset.get(asset_contract).try_read().unwrap_or(U128::zero());
    require(
        debt_to_offset <= total_usdf_deposits,
        "StabilityPool: Debt offset exceeds total USDF deposits",
    );
    let mut usdf_loss_per_unit_staked: U128 = U128::zero();
    if (debt_to_offset == total_usdf_deposits) {
        usdf_loss_per_unit_staked = U128::from(DECIMAL_PRECISION);
        storage.last_usdf_error_offset.write(U128::zero());
    } else {
        let usdf_loss_per_unit_staked_numerator: U128 = U128::from(debt_to_offset) * U128::from(DECIMAL_PRECISION) - storage.last_usdf_error_offset.read();
        usdf_loss_per_unit_staked = usdf_loss_per_unit_staked_numerator / U128::from(total_usdf_deposits) + U128::from(1u64);
        storage
            .last_usdf_error_offset
            .write(
                usdf_loss_per_unit_staked * U128::from(total_usdf_deposits) - usdf_loss_per_unit_staked_numerator,
            );
    }
    let asset_gain_per_unit_staked = asset_numerator / U128::from(total_usdf_deposits);
    storage
        .last_asset_error_offset
        .insert(
            asset_contract,
            asset_numerator - (asset_gain_per_unit_staked * U128::from(total_usdf_deposits)),
        );
    return (asset_gain_per_unit_staked, usdf_loss_per_unit_staked);
}
#[storage(read, write)]
fn update_reward_sum_and_product(
    asset_gain_per_unit_staked: U128,
    usdf_loss_per_unit_staked: U128,
    asset: AssetId,
) {
    let current_p = storage.p.read();
    let mut new_p: U128 = U128::zero();
    let new_product_factor = U128::from(DECIMAL_PRECISION) - usdf_loss_per_unit_staked;
    let current_epoch = storage.current_epoch.read();
    let current_scale = storage.current_scale.read();
    let current_s = storage.epoch_to_scale_to_sum.get((current_epoch, current_scale, asset)).try_read().unwrap_or(U128::zero());
    let marginal_asset_gain: U128 = asset_gain_per_unit_staked * current_p;
    let new_sum = current_s + marginal_asset_gain;
    storage
        .epoch_to_scale_to_sum
        .insert((current_epoch, current_scale, asset), new_sum);
    if (new_product_factor == U128::zero()) {
        storage
            .current_epoch
            .write(storage.current_epoch.read() + 1);
        storage.current_scale.write(0);
        new_p = U128::from(DECIMAL_PRECISION);
    } else if (current_p * new_product_factor / U128::from(DECIMAL_PRECISION) < U128::from(SCALE_FACTOR))
    {
        new_p = current_p * new_product_factor * U128::from(SCALE_FACTOR) / U128::from(DECIMAL_PRECISION);
        storage
            .current_scale
            .write(storage.current_scale.read() + 1);
    } else {
        new_p = current_p * new_product_factor / U128::from(DECIMAL_PRECISION);
    }
    require(new_p > U128::zero(), "StabilityPool: New p is 0");
    storage.p.write(new_p);
}
#[storage(read, write)]
fn internal_move_offset_coll_and_debt(
    coll_to_add: u64,
    debt_to_offset: u64,
    asset_contract: AssetId,
) {
    let active_pool = abi(ActivePool, storage.active_pool_contract.read().bits());
    let usdf_contract = abi(SRC3, storage.usdf_contract.read().bits());
    internal_decrease_usdf(debt_to_offset);
    internal_increase_asset(coll_to_add, asset_contract);
    active_pool.decrease_usdf_debt(debt_to_offset, asset_contract);
    usdf_contract
        .burn {
            coins: debt_to_offset,
            asset_id: storage.usdf_asset_id.read().bits(),
        }(SubId::zero(), debt_to_offset);
    active_pool.send_asset(
        Identity::ContractId(ContractId::this()),
        coll_to_add,
        asset_contract,
    );
}
