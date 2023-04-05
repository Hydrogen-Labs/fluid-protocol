contract;

dep data_structures;
dep interface;
dep utils;

use data_structures::{Asset, VestingSchedule};
use interface::VestingContract;
use utils::{calculate_redeemable_amount, is_valid_vesting_schedule};
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
    admin: Identity = Identity::Address(Address::from(ZERO_B256)),
    vesting_schedules: StorageMap<Identity, Option<VestingSchedule>> = StorageMap {},
    vesting_addresses: StorageVec<Identity> = StorageVec {},
    asset: Asset = Asset::new(ContractId::from(ZERO_B256), 0),
}

#[storage(read)]
fn validate_admin() {
    let sender = msg_sender().unwrap();
    require(storage.admin == sender, "Access denied");
}

impl VestingContract for Contract {
    #[storage(write, read)]
    fn constructor(
        admin: Identity,
        schedules: Vec<VestingSchedule>,
        asset: Asset,
    ) {
        storage.admin = admin;
        // TODO Check that there are sufficient funds to cover all vesting schedules
        let mut i = 0;

        while i < schedules.len() {
            let schedule = schedules.get(i).unwrap();
            require(is_valid_vesting_schedule(schedule), "Invalid vesting schedule");

            let existing_schedule = storage.vesting_schedules.get(schedule.recipient);
            require(existing_schedule.is_none(), "Schedule already exists");

            storage.vesting_schedules.insert(schedule.recipient, Option::Some(schedule));
            storage.vesting_addresses.push(schedule.recipient);
            i += 1;
        }

        storage.asset = asset;
    }

    #[storage(read, write)]
    fn claim_vested_tokens(address: Identity) {
        // TODO add re entry guard
        let mut schedule = storage.vesting_schedules.get(address).unwrap();
        // TODO switch back to timestamp, but currently not supported by Fuel for unit testing
        let now = height();

        let unclaimed = calculate_redeemable_amount(now, schedule);
        require(unclaimed > 0, "Nothing to redeem");

        transfer(unclaimed, storage.asset.id, address);
        schedule.claimed_amount += unclaimed;

        storage.vesting_schedules.insert(address, Option::Some(schedule));
    }

    #[storage(read, write)]
    fn revoke_vesting_schedule(address: Identity) {
        validate_admin();
        // TODO add re entry guard
        let schedule = storage.vesting_schedules.get(address).unwrap();

        let unclaimed = schedule.total_amount - schedule.claimed_amount;
        require(unclaimed > 0, "Nothing to revoke");
        storage.vesting_schedules.insert(address, Option::None);

        transfer(unclaimed, storage.asset.id, storage.admin);
    }

    #[storage(read)]
    fn get_vesting_schedule(address: Identity) -> Option<VestingSchedule> {
        return storage.vesting_schedules.get(address);
    }

    #[storage(read)]
    fn get_redeemable_amount(at_timestamp: u64, address: Identity) -> u64 {
        let schedule = storage.vesting_schedules.get(address).unwrap();

        return calculate_redeemable_amount(at_timestamp, schedule);
    }

    #[storage(read)]
    fn get_current_time() -> u64 {
        return timestamp();
    }

 
    // TODO waiting for Fuel to enable Vector outputs 
    // #[storage(read)]
    // fn get_vesting_addresses() -> Vec<Identity> {
    //     let mut i = 0;
    //     let mut addresses: Vec<Identity> = Vec::new();
    //     while i < storage.vesting_addresses.len() {
    //         let address = storage.vesting_addresses.get(i).unwrap();
    //         addresses.push(address);
    //         i += 1;
    //     }
    //     return addresses;
    // }
}
