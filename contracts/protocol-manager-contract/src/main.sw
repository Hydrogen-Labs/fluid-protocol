contract;
// This contract, ProtocolManager, is responsible for managing the overall protocol operations.
// It acts as a central coordinator for various critical contracts and interfaces within the system.
//
// Key functionalities include:
// - Initializing the protocol by registering asset contracts and setting up necessary connections
// - Administering the ownership and access control mechanisms
// - Facilitating the redemption process for users
// - Interfacing with the Stability Pool for FPT issuance
mod data_structures;
use ::data_structures::{AssetContracts, AssetInfo, RedemptionTotals};
use libraries::stability_pool_interface::StabilityPool;
use libraries::trove_manager_interface::TroveManager;
use libraries::trove_manager_interface::data_structures::SingleRedemptionValues;
use libraries::borrow_operations_interface::BorrowOperations;
use libraries::sorted_troves_interface::SortedTroves;
use libraries::active_pool_interface::ActivePool;
use libraries::default_pool_interface::DefaultPool;
use libraries::coll_surplus_pool_interface::CollSurplusPool;
use libraries::oracle_interface::Oracle;
use libraries::protocol_manager_interface::ProtocolManager;
use libraries::usdf_token_interface::USDFToken;
use libraries::fpt_staking_interface::FPTStaking;
use libraries::fluid_math::*;
use sway_libs::ownership::*;
use standards::{src3::SRC3, src5::*,};
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
    storage::storage_vec::*,
};
configurable {
    /// Initializer identity
    INITIALIZER: Identity = Identity::Address(Address::zero()),
}
storage {
    borrow_operations_contract: ContractId = ContractId::zero(),
    fpt_staking_contract: ContractId = ContractId::zero(),
    usdf_token_contract: ContractId = ContractId::zero(),
    stability_pool_contract: ContractId = ContractId::zero(),
    coll_surplus_pool_contract: ContractId = ContractId::zero(),
    default_pool_contract: ContractId = ContractId::zero(),
    active_pool_contract: ContractId = ContractId::zero(),
    sorted_troves_contract: ContractId = ContractId::zero(),
    asset_contracts: StorageMap<AssetId, AssetContracts> = StorageMap::<AssetId, AssetContracts> {},
    assets: StorageVec<AssetId> = StorageVec {},
    is_initialized: bool = false,
    lock_redeem_collateral: bool = false,
}
impl ProtocolManager for Contract {
    #[storage(read, write)]
    fn initialize(
        borrow_operations: ContractId,
        stability_pool: ContractId,
        fpt_staking: ContractId,
        usdf_token: ContractId,
        coll_surplus_pool: ContractId,
        default_pool: ContractId,
        active_pool: ContractId,
        sorted_troves: ContractId,
        initial_owner: Identity,
    ) {
        require(
            msg_sender()
                .unwrap() == INITIALIZER,
            "ProtocolManager: Caller is not initializer",
        );
        require(
            storage
                .is_initialized
                .read() == false,
            "ProtocolManager: Already initialized",
        );
        initialize_ownership(initial_owner);
        storage.borrow_operations_contract.write(borrow_operations);
        storage.fpt_staking_contract.write(fpt_staking);
        storage.stability_pool_contract.write(stability_pool);
        storage.usdf_token_contract.write(usdf_token);
        storage.coll_surplus_pool_contract.write(coll_surplus_pool);
        storage.default_pool_contract.write(default_pool);
        storage.active_pool_contract.write(active_pool);
        storage.sorted_troves_contract.write(sorted_troves);
        storage.is_initialized.write(true);
    }
    #[storage(read, write)]
    fn register_asset(
        asset_address: AssetId,
        trove_manager: ContractId,
        oracle: ContractId,
    ) {
        only_owner();
        require_asset_not_registered(asset_address);
        let stability_pool = abi(StabilityPool, storage.stability_pool_contract.read().bits());
        let borrow_operations = abi(BorrowOperations, storage.borrow_operations_contract.read().bits());
        let usdf_token = abi(USDFToken, storage.usdf_token_contract.read().bits());
        let fpt_staking = abi(FPTStaking, storage.fpt_staking_contract.read().bits());
        let coll_surplus_pool = abi(CollSurplusPool, storage.coll_surplus_pool_contract.read().bits());
        let default_pool = abi(DefaultPool, storage.default_pool_contract.read().bits());
        let active_pool = abi(ActivePool, storage.active_pool_contract.read().bits());
        let sorted_troves = abi(SortedTroves, storage.sorted_troves_contract.read().bits());
        storage
            .asset_contracts
            .insert(
                asset_address,
                AssetContracts {
                    trove_manager,
                    oracle,
                    asset_address,
                },
            );
        storage.assets.push(asset_address);
        borrow_operations.add_asset(asset_address, trove_manager, oracle);
        coll_surplus_pool.add_asset(asset_address, Identity::ContractId(trove_manager));
        active_pool.add_asset(asset_address, Identity::ContractId(trove_manager));
        default_pool.add_asset(asset_address, Identity::ContractId(trove_manager));
        stability_pool.add_asset(trove_manager, asset_address, oracle);
        sorted_troves.add_asset(asset_address, trove_manager);
        fpt_staking.add_asset(asset_address);
        usdf_token.add_trove_manager(trove_manager);
    }
    #[storage(read, write)]
    fn renounce_admin() {
        only_owner();
        renounce_ownership();
    }
    #[storage(read, write), payable]
    fn redeem_collateral(
        max_iterations: u64,
        partial_redemption_hint: u64,
        upper_partial_hint: Identity,
        lower_partial_hint: Identity,
    ) {
        require(
            storage
                .lock_redeem_collateral
                .read() == false,
            "ProtocolManager: Redeem collateral is locked",
        );
        storage.lock_redeem_collateral.write(true);

        require_valid_usdf_id();
        require(
            msg_amount() > 0,
            "ProtocolManager: Redemption amount must be greater than 0",
        );
        let usdf_contract_cache = storage.usdf_token_contract.read();
        let fpt_staking_contract_cache = storage.fpt_staking_contract.read();
        let usdf = abi(SRC3, usdf_contract_cache.bits());
        let sorted_troves = abi(SortedTroves, storage.sorted_troves_contract.read().bits());
        let active_pool = abi(ActivePool, storage.active_pool_contract.read().bits());
        let fpt_staking = abi(FPTStaking, fpt_staking_contract_cache.bits());
        let mut assets_info = get_all_assets_info();
        let mut remaining_usdf = msg_amount();
        let (mut current_borrower, mut index) = find_min_borrower(assets_info.current_borrowers, assets_info.current_crs);
        let mut remaining_iterations = max_iterations;

        // Iterate through troves, redeeming collateral until conditions are met
        while (current_borrower != null_identity_address() && remaining_usdf > 0 && remaining_iterations > 0) {
            let contracts_cache = assets_info.asset_contracts.get(index).unwrap();
            let trove_manager_contract = abi(TroveManager, contracts_cache.trove_manager.bits());
            let price = assets_info.prices.get(index).unwrap();
            let mut totals = assets_info.redemption_totals.get(index).unwrap();
            remaining_iterations -= 1;
            let next_user_to_check = sorted_troves.get_prev(current_borrower, contracts_cache.asset_address);

            // Check if this is the last trove for this asset
            let next_trove = sorted_troves.get_prev(current_borrower, contracts_cache.asset_address);
            if (next_trove == null_identity_address()) {
                // This is the last trove, skip it and move to next asset
                assets_info.current_crs.set(index, u64::max());
                assets_info
                    .current_borrowers
                    .set(index, null_identity_address());
                assets_info.redemption_totals.set(index, totals);
                let next_borrower = find_min_borrower(assets_info.current_borrowers, assets_info.current_crs);
                current_borrower = next_borrower.0;
                index = next_borrower.1;
                continue;
            }

            // Apply pending rewards to ensure up-to-date trove state
            trove_manager_contract.apply_pending_rewards(current_borrower);

            // Attempt to redeem collateral from the current trove
            let single_redemption = trove_manager_contract.redeem_collateral_from_trove(
                current_borrower,
                remaining_usdf,
                price,
                partial_redemption_hint,
                upper_partial_hint,
                lower_partial_hint,
            );

            // Break if partial redemption was cancelled
            if (single_redemption.cancelled_partial) {
                break;
            }

            // Update totals and remaining USDF
            totals.total_usdf_to_redeem += single_redemption.usdf_lot;
            totals.total_asset_drawn += single_redemption.asset_lot;
            remaining_usdf -= single_redemption.usdf_lot;

            let mut next_cr = u64::max();
            if (next_user_to_check != null_identity_address()) {
                next_cr = trove_manager_contract.get_current_icr(next_user_to_check, price);
            }
            assets_info.current_crs.set(index, next_cr);
            assets_info.current_borrowers.set(index, next_user_to_check);
            assets_info.redemption_totals.set(index, totals);
            let next_borrower = find_min_borrower(assets_info.current_borrowers, assets_info.current_crs);
            current_borrower = next_borrower.0;
            index = next_borrower.1;
        }

        let mut total_usdf_redeemed = 0;
        let mut ind = 0;

        // Process redemptions for each asset
        while (ind < assets_info.assets.len()) {
            let contracts_cache = assets_info.asset_contracts.get(ind).unwrap();

            let mut totals = assets_info.redemption_totals.get(ind).unwrap();

            if (totals.total_usdf_to_redeem == 0) {
                ind += 1;
                continue;
            }

            // Calculate redemption fee and amount to send to redeemer
            totals.asset_fee = fm_compute_redemption_fee(totals.total_asset_drawn);
            totals.asset_to_send_to_redeemer = totals.total_asset_drawn - totals.asset_fee;

            // Send redemption fee to FPT stakers
            active_pool.send_asset(
                Identity::ContractId(fpt_staking_contract_cache),
                totals
                    .asset_fee,
                contracts_cache
                    .asset_address,
            );
            fpt_staking.increase_f_asset(totals.asset_fee, assets_info.assets.get(ind).unwrap());

            // Update total USDF redeemed and decrease USDF debt
            total_usdf_redeemed += totals.total_usdf_to_redeem;
            active_pool.decrease_usdf_debt(totals.total_usdf_to_redeem, contracts_cache.asset_address);

            // Send redeemed collateral to the user
            active_pool.send_asset(
                msg_sender()
                    .unwrap(),
                totals
                    .asset_to_send_to_redeemer,
                contracts_cache
                    .asset_address,
            );

            ind += 1;
        }

        // Burn the redeemed USDF
        usdf
            .burn {
                coins: total_usdf_redeemed,
                asset_id: AssetId::new(usdf_contract_cache, SubId::zero()).bits(),
            }(SubId::zero(), total_usdf_redeemed);

        // Return any remaining USDF to the redeemer
        if (remaining_usdf > 0) {
            transfer(
                msg_sender()
                    .unwrap(),
                AssetId::new(usdf_contract_cache, SubId::zero()),
                remaining_usdf,
            );
        }

        storage.lock_redeem_collateral.write(false);
    }
    #[storage(read, write)]
    fn transfer_owner(new_owner: Identity) {
        only_owner();
        transfer_ownership(new_owner);
    }
}

impl SRC5 for Contract {
    #[storage(read)]
    fn owner() -> State {
        _owner()
    }
}
// --- Helper functions ---
#[storage(read)]
fn require_valid_usdf_id() {
    require(
        msg_asset_id() == AssetId::new(storage.usdf_token_contract.read(), SubId::zero()),
        "ProtocolManager: Invalid asset being transfered",
    );
}

// Get information about all assets in the system
#[storage(read)]
fn get_all_assets_info() -> AssetInfo {
    let mut assets: Vec<AssetId> = Vec::new();
    let mut asset_contracts: Vec<AssetContracts> = Vec::new();
    let mut prices: Vec<u64> = Vec::new();
    let mut system_debt: Vec<u64> = Vec::new();
    let mut redemption_totals: Vec<RedemptionTotals> = Vec::new();
    let mut current_borrowers: Vec<Identity> = Vec::new();
    let mut current_crs: Vec<u64> = Vec::new();
    let sorted_troves = abi(SortedTroves, storage.sorted_troves_contract.read().bits());
    let length = storage.assets.len();
    let mut ind = 0;
    while (ind < length) {
        assets.push(storage.assets.get(ind).unwrap().read());
        asset_contracts.push(storage.asset_contracts.get(assets.get(ind).unwrap()).read());
        ind += 1;
    }
    let mut i = 0;
    while (i < length) {
        let oracle = abi(Oracle, asset_contracts.get(i).unwrap().oracle.into());
        let trove_manager = abi(TroveManager, asset_contracts.get(i).unwrap().trove_manager.into());
        let asset = assets.get(i).unwrap();
        let price = oracle.get_price();
        let mut current_borrower = sorted_troves.get_last(asset);
        let mut current_cr = u64::max();
        if (current_borrower != null_identity_address()) {
            current_cr = trove_manager.get_current_icr(current_borrower, price);
        }
        prices.push(price);
        system_debt.push(trove_manager.get_entire_system_debt());
        redemption_totals.push(RedemptionTotals::default());
        while (current_borrower != null_identity_address() && current_cr < MCR) {
            current_borrower = sorted_troves.get_prev(current_borrower, asset);
            current_cr = trove_manager.get_current_icr(current_borrower, price);
        }
        current_borrowers.push(current_borrower);
        current_crs.push(current_cr);
        i += 1;
    }
    AssetInfo {
        assets: assets,
        asset_contracts: asset_contracts,
        prices: prices,
        system_debts: system_debt,
        redemption_totals: redemption_totals,
        current_borrowers: current_borrowers,
        current_crs: current_crs,
    }
}
// Find the borrower with the lowest collateral ratio
fn find_min_borrower(current_borrowers: Vec<Identity>, current_crs: Vec<u64>) -> (Identity, u64) {
    let mut min_borrower = current_borrowers.get(0).unwrap();
    let mut min_cr = current_crs.get(0).unwrap();
    let mut min_index = 0;
    let mut i = 1;
    while (i < current_borrowers.len()) {
        if (current_crs.get(i).unwrap() < min_cr) {
            min_borrower = current_borrowers.get(i).unwrap();
            min_cr = current_crs.get(i).unwrap();
            min_index = i;
        }
        i += 1;
    }
    (min_borrower, min_index)
}

#[storage(read)]
fn require_asset_not_registered(asset_id: AssetId) {
    let length = storage.assets.len();
    let mut i = 0;
    while (i < length) {
        if (storage.assets.get(i).unwrap().read() == asset_id) {
            revert(0);
        }
        i += 1;
    }
}
