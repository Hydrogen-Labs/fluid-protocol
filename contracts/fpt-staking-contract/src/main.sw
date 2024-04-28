contract;

use libraries::fluid_math::{
    DECIMAL_PRECISION,
    fm_min,
    fm_multiply_ratio,
    null_contract,
    null_identity_address,
    ZERO_B256,
};
use libraries::fpt_staking_interface::{FPTStaking, ReadStorage};
use std::{
    asset::transfer,
    auth::msg_sender,
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
storage {
    valid_assets: StorageVec<AssetId> = StorageVec {},
    stakes: StorageMap<Identity, u64> = StorageMap::<Identity, u64> {},
    usdf_snapshot: StorageMap<Identity, u64> = StorageMap::<Identity, u64> {},
    asset_snapshot: StorageMap<(Identity, AssetId), u64> = StorageMap::<(Identity, AssetId), u64> {},
    f_asset: StorageMap<AssetId, u64> = StorageMap::<AssetId, u64> {},
    f_usdf: u64 = 0,
    total_fpt_staked: u64 = 0,
    protocol_manager_address: ContractId = ContractId::from(ZERO_B256),
    borrower_operations_address: ContractId = ContractId::from(ZERO_B256),
    fpt_address: AssetId = AssetId::from(ZERO_B256),
    usdf_address: AssetId = AssetId::from(ZERO_B256),
    is_initialized: bool = false,
}

impl FPTStaking for Contract {
    #[storage(read, write)]
    fn initialize(
        protocol_manager_address: ContractId,
        borrower_operations_address: ContractId,
        fpt_address: AssetId,
        usdf_address: AssetId,
    ) {
        require(
            storage
                .is_initialized
                .read() == false,
            "Contract is already initialized",
        );
        storage
            .protocol_manager_address
            .write(protocol_manager_address);
        storage
            .borrower_operations_address
            .write(borrower_operations_address);
        storage.fpt_address.write(fpt_address);
        storage.usdf_address.write(usdf_address);
        storage.is_initialized.write(true);
    }

    #[storage(read)]
    fn get_storage() -> ReadStorage {
        return ReadStorage {
            f_usdf: storage.f_usdf.read(),
            total_fpt_staked: storage.total_fpt_staked.read(),
            protocol_manager_address: storage.protocol_manager_address.read(),
            borrower_operations_address: storage.borrower_operations_address.read(),
            fpt_address: storage.fpt_address.read(),
            usdf_address: storage.usdf_address.read(),
            is_initialized: storage.is_initialized.read(),
        }
    }

    #[storage(read, write), payable]
    fn stake() {
        let id = msg_sender().unwrap();

        require_fpt_is_valid_and_non_zero();

        let amount = msg_amount();

        let current_stake = storage.stakes.get(id).try_read().unwrap_or(0);

        if (current_stake != 0) {
            let usdf_gain = internal_get_pending_usdf_gain(id);
            internal_send_usdf_gain_to_user(usdf_gain);

            internal_send_asset_gain_to_user(id);
        }

        update_user_snapshots(id);

        let new_stake = current_stake + amount;
        storage.stakes.insert(id, new_stake); //overwrite previous balance
        storage
            .total_fpt_staked
            .write(storage.total_fpt_staked.read() + amount);
    }

    #[storage(read, write)]
    fn unstake(amount: u64) {
        let id = msg_sender().unwrap();

        let current_stake = storage.stakes.get(id).try_read().unwrap_or(0);
        require_user_has_stake(current_stake, amount);

        let usdf_gain = internal_get_pending_usdf_gain(id);
        internal_send_usdf_gain_to_user(usdf_gain);
        internal_send_asset_gain_to_user(id);

        update_user_snapshots(id);

        if (amount > 0) {
            let amount_to_withdraw = fm_min(amount, current_stake);
            let new_stake = current_stake - amount_to_withdraw;
            storage.stakes.insert(id, new_stake); //overwrite previous balance
            storage
                .total_fpt_staked
                .write(storage.total_fpt_staked.read() - amount_to_withdraw);

            if (amount_to_withdraw > 0) {
                // transfer the FPT tokens to the user
                transfer(
                    msg_sender()
                        .unwrap(),
                    storage
                        .fpt_address
                        .read(),
                    amount_to_withdraw,
                );
            }
        }
    }

    // called from the protocol manager contract in the `register_asset` fn
    #[storage(read, write)]
    fn add_asset(asset_address: AssetId) {
        require_is_protocol_manager();
        storage.valid_assets.push(asset_address);
        storage.f_asset.insert(asset_address, 0);
    }
    
    #[storage(read)]
    fn get_staking_balance(id: Identity) -> u64 {
        storage.stakes.get(id).try_read().unwrap_or(0);
    }

    #[storage(read)]
    fn get_pending_asset_gain(id: Identity, asset_address: AssetId) -> u64 {
        internal_get_pending_asset_gain(id, asset_address)
    }

    #[storage(read)]
    fn get_pending_usdf_gain(id: Identity) -> u64 {
        internal_get_pending_usdf_gain(id)
    }

    #[storage(read, write)]
    fn increase_f_usdf(usdf_fee_amount: u64) {
        require_is_borrower_operations();
        if (storage.total_fpt_staked.read() > 0) {
            let usdf_fee_per_fpt_staked = fm_multiply_ratio(
                usdf_fee_amount,
                DECIMAL_PRECISION,
                storage
                    .total_fpt_staked
                    .read(),
            );
            storage
                .f_usdf
                .write(storage.f_usdf.read() + usdf_fee_per_fpt_staked);
        }
    }

    #[storage(read, write)]
    fn increase_f_asset(asset_fee_amount: u64, asset_address: AssetId) {
        require_is_protocol_manager(); // we have redeem function in protocol manager, not trove manager in liquity
        if (storage.total_fpt_staked.read() > 0) {
            let asset_fee_per_fpt_staked = fm_multiply_ratio(
                asset_fee_amount,
                DECIMAL_PRECISION,
                storage
                    .total_fpt_staked
                    .read(),
            );
            let mut new_f_asset = storage.f_asset.get(asset_address).read() + asset_fee_per_fpt_staked;
            storage.f_asset.insert(asset_address, new_f_asset);
        }
    }
}

#[storage(read)]
fn internal_get_pending_asset_gain(id: Identity, asset_address: AssetId) -> u64 {
    let f_asset_snapshot = storage.asset_snapshot.get((id, asset_address)).try_read().unwrap_or(0);
    let asset_gain = fm_multiply_ratio(
        storage
            .stakes
            .get(id)
            .try_read()
            .unwrap_or(0),
        storage
            .f_asset
            .get(asset_address)
            .try_read()
            .unwrap_or(0) - f_asset_snapshot,
        DECIMAL_PRECISION,
    );
    return asset_gain
}

#[storage(read)]
fn internal_get_pending_usdf_gain(id: Identity) -> u64 {
    let f_usdf_snapshot = storage.usdf_snapshot.get(id).try_read().unwrap_or(0);
    let usdf_gain = fm_multiply_ratio(
        storage
            .stakes
            .get(id)
            .try_read()
            .unwrap_or(0),
        storage
            .f_usdf
            .read() - f_usdf_snapshot,
        DECIMAL_PRECISION,
    );
    return usdf_gain
}

#[storage(read, write)]
fn update_user_snapshots(id: Identity) {
    storage.usdf_snapshot.insert(id, storage.f_usdf.read());

    let mut ind = 0;
    while ind < storage.valid_assets.len() {
        let current_asset_address = storage.valid_assets.get(ind).unwrap().read();
        let f_asset = storage.f_asset.get(current_asset_address).try_read().unwrap_or(0);
        storage
            .asset_snapshot
            .insert((id, current_asset_address), f_asset);
        ind += 1;
    }
}

fn require_user_has_stake(current_stake_amount: u64, unstake_amount: u64) {
    require(
        current_stake_amount > 0,
        "User must have stake greater than 0",
    );
    require(
        current_stake_amount >= unstake_amount,
        "Cannot unstake more than current staked amount",
    );
}

#[storage(read)]
fn require_is_protocol_manager() {
    let protocol_manager = Identity::ContractId(storage.protocol_manager_address.read());
    require(
        msg_sender()
            .unwrap() == protocol_manager,
        "Caller is not the protocol manager",
    );
}

#[storage(read)]
fn require_is_borrower_operations() {
    let borrower_operations = Identity::ContractId(storage.borrower_operations_address.read());
    require(
        msg_sender()
            .unwrap() == borrower_operations,
        "Caller is not the Borrower Operations",
    );
}

#[storage(read)]
fn require_fpt_is_valid_and_non_zero() {
    require(
        storage
            .fpt_address
            .read() == msg_asset_id(),
        "FPT contract not initialized, or wrong token",
    );
    require(msg_amount() > 0, "FPT amount must be greater than 0");
}

#[storage(read)]
fn internal_send_asset_gain_to_user(id: Identity) {
    // when fuel adds a .contains or .indexOf for StorageVec, double check asset address is in valid_assets here
    let mut ind = 0;
    while ind < storage.valid_assets.len() {
        let current_asset_address = storage.valid_assets.get(ind).unwrap().read();
        let asset_gain = internal_get_pending_asset_gain(id, current_asset_address);
        if (asset_gain > 0) {
            transfer(msg_sender().unwrap(), current_asset_address, asset_gain);
        }
        ind += 1;
    }
}

#[storage(read)]
fn internal_send_usdf_gain_to_user(amount: u64) {
    if (amount > 0) {
        transfer(msg_sender().unwrap(), storage.usdf_address.read(), amount);
    }
}
