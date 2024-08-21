contract;

use libraries::mock_oracle_interface::RedstoneCore;
use std::hash::Hash;

storage {
    timestamp: u64 = 0,
    prices: StorageMap<u256, u256> = StorageMap {},
}

impl RedstoneCore for Contract {
    #[storage(read)]
    fn read_prices(feed_ids: Vec<u256>) -> Vec<u256> {
        let mut prices = Vec::with_capacity(feed_ids.len());
        let mut index = 0;
        while (index < feed_ids.len()) {
            let entry = feed_ids.get(index).unwrap();
            prices.push(storage.prices.get(entry).read());
            index += 1;
        }
        prices
    }

    // Testing only, not the actual function signature of redstone
    #[storage(write)]
    fn write_prices(feed: Vec<(u256, u256)>) {
        let mut index = 0;
        while (index < feed.len()) {
            let entry = feed.get(index).unwrap();
            storage.prices.insert(entry.0, entry.1);
            index += 1;
        }
    }

    #[storage(read)]
    fn read_timestamp() -> u64 {
        storage.timestamp.read()
    }

    #[storage(write)]
    fn set_timestamp(time: u64) {
        storage.timestamp.write(time)
    }
}
