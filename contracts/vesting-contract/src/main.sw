contract;

mod data_structures;
mod interface;
mod utils;

use ::data_structures::{VestingSchedule};
use ::interface::VestingContract;
use ::utils::{calculate_redeemable_amount, is_valid_vesting_schedule};

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
    hash::Hash,
    logging::log,
    storage::storage_vec::*,
    token::transfer,
};

const ZERO_B256 = 0x0000000000000000000000000000000000000000000000000000000000000000;

storage {
    vesting_schedules: StorageMap<Identity, VestingSchedule> = StorageMap::<Identity, VestingSchedule> {},
    vesting_addresses: StorageVec<Identity> = StorageVec {},
    asset: AssetId = AssetId::from(ZERO_B256),
    is_initialized: bool = false,
    // timestamp is used for testing purposes only, as Fuel does not support timestamp currently in integration tests
    debug: bool = false,
    debug_timestamp: u64 = 0,
}

impl VestingContract for Contract {
    #[storage(write, read)]
    fn constructor(
        asset: AssetId,
        schedules: Vec<VestingSchedule>,
        debugging: bool,
    ) {
        require(!storage.is_initialized.read(), "Contract is already initialized");
        // TODO Check that there are sufficient funds to cover all vesting schedules
        storage.asset.write(asset);
        storage.debug.write(debugging);

        let mut i = 0;
        while i < schedules.len() {
            let schedule = schedules.get(i).unwrap();
            require(is_valid_vesting_schedule(schedule), "Invalid vesting schedule");

            match storage.vesting_schedules.get(schedule.recipient).try_read() {
                Some(_) => require(false, "Schedule already exists"),
                None => {}
            }

            storage.vesting_schedules.insert(schedule.recipient, schedule);
            storage.vesting_addresses.push(schedule.recipient);
            i += 1;
        }

        storage.is_initialized.write(true);
    }

    #[storage(read, write)]
    fn claim_vested_tokens() {
        let address = msg_sender().unwrap();
        // TODO add re entry guard
        let mut schedule = storage.vesting_schedules.get(address).read();
        // TODO switch back to timestamp, but currently not supported by Fuel for unit testing
        let now = internal_get_current_time();

        let currently_unclaimed = calculate_redeemable_amount(now, schedule);
        require(currently_unclaimed > 0, "Nothing to redeem");
        schedule.claimed_amount += currently_unclaimed;
        storage.vesting_schedules.insert(address, schedule);

        transfer(address, storage.asset.read(), currently_unclaimed);
    }

    #[storage(read)]
    fn get_vesting_schedule(address: Identity) -> VestingSchedule {
        return storage.vesting_schedules.get(address).read();
    }

    #[storage(read)]
    fn get_redeemable_amount(at_timestamp: u64, address: Identity) -> u64 {
        let schedule = storage.vesting_schedules.get(address).read();

        return calculate_redeemable_amount(at_timestamp, schedule);
    }

    #[storage(read)]
    fn get_current_time() -> u64 {
        return internal_get_current_time();
    }

    #[storage(write, read)]
    fn set_current_time(time: u64) {
        require(storage.debug.read(), "Debugging must be enabled to set current time");
        storage.debug_timestamp.write(time);
    }
}

#[storage(read)]
fn internal_get_current_time() -> u64 {
    if storage.debug.read() {
        return storage.debug_timestamp.read();
    } else {
        return timestamp();
    }
}
