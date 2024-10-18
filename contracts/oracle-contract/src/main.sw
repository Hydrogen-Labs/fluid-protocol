contract;
// This contract, Oracle, serves as an interface to query asset prices from either Pyth or Redstone oracles.
//
// Key functionalities include:
// - Providing a unified interface to fetch price data from different oracle sources
// - Converting price data from different precisions to a standardized format
// - Implementing safeguards against potential precision issues
// - Prioritizing price sources based on availability and recency
//
// Price Priority:
// 1. Pyth Oracle: The contract first attempts to fetch the price from the Pyth oracle.
// 2. Redstone Oracle: If the Pyth price is unavailable or outdated, the contract falls back to the Redstone oracle.
//
// This prioritization ensures that the most reliable and recent price data is used,
// enhancing the overall stability and accuracy of the Fluid Protocol.


use libraries::{
    fluid_math::convert_precision,
    oracle_interface::RedstoneCore,
    oracle_interface::{
        Oracle,
        Price,
    },
    oracle_interface::{
        PythCore,
        PythPrice,
        PythPriceFeedId,
    },
};
use std::{block::timestamp, constants::ZERO_B256,};

// Hack because of Sway Compiler consuming < 64GB RAM in library import location
impl From<PythPrice> for Price {
    fn from(p: PythPrice) -> Self {
        Self {
            value: convert_precision(p.price, PYTH_PRECISION),
            time: p.publish_time,
        }
    }
}

// Hack: Sway does not provide a downcast to u64
// If redstone provides a strangely high u256 which shouldn't be cast down
// then other parts of the code must be adjusted to use u256
impl u256 {
    fn to_u64(self) -> u64 {
        let (_a, _b, _c, d): (u64, u64, u64, u64) = asm(r1: self) {
            r1: (u64, u64, u64, u64)
        };

        d
    }
}

configurable {
    /// Contract Address
    PYTH: ContractId = ContractId::zero(),
    /// Price feed to query
    PYTH_PRICE_ID: PythPriceFeedId = ZERO_B256,
    /// Precision of value returned by Pyth
    PYTH_PRECISION: u8 = 9,
    /// Contract Address
    REDSTONE: Option<ContractId> = None,
    /// Price feed to query
    REDSTONE_PRICE_ID: u256 = u256::min(),
    /// Precision of value returned by Redstone
    REDSTONE_PRECISION: u8 = 9,
    /// Timeout in seconds
    DEBUG: bool = false,
}
// Timeout period for considering oracle data as stale (4 hours in seconds)
const TIMEOUT: u64 = 14400;

storage {
    /// The last valid price from either Pyth or Redstone
    price: Price = Price {
        value: 0,
        time: 0,
    },
    // Used for simulating different timestamps during testing
    debug_timestamp: u64 = 0,
}

impl Oracle for Contract {
    #[storage(read, write)]
    fn get_price() -> u64 {
        let mut pyth_price = abi(PythCore, PYTH.bits()).price_unsafe(PYTH_PRICE_ID);
        pyth_price = pyth_price_with_fuel_vm_precision_adjustment(pyth_price, FUEL_DECIMAL_REPRESENTATION);

        // If Redstone is not configured, return the Pyth price regardless of staleness or confidence 
        // price_unsafe retunrs the last price regardless of staleness with no reverts
        if REDSTONE.is_none() {
            return pyth_price.price;
        }
        // Determine the current timestamp based on debug mode
        let current_time = match DEBUG {
            true => storage.debug_timestamp.read(),
            false => timestamp(),
        };
        // Read the last stored valid price
        let last_price = storage.price.read();

        // Step 1: Query the Pyth oracle (primary source)

        // Check if Pyth data is stale or outside confidence
        if is_pyth_price_stale_or_outside_confidence(pyth_price, current_time) {
            // Step 2: Pyth is stale or outside confidence, query Redstone oracle (fallback source)
            let mut feed = Vec::with_capacity(1);
            feed.push(REDSTONE_PRICE_ID);

            // Fuel Bug workaround: trait coherence
            let id = REDSTONE.unwrap().bits();
            let redstone = abi(RedstoneCore, id);
            let redstone_prices = redstone.read_prices(feed);
            let redstone_timestamp = redstone.read_timestamp();
            let redstone_price = convert_precision(redstone_prices.get(0).unwrap().to_u64(), REDSTONE_PRECISION);

            // Check if Redstone data is also stale
            if current_time - redstone_timestamp > TIMEOUT {
                // Both oracles are stale, use the most recent data available
                if redstone_timestamp <= pyth_price.publish_time {
                    // Pyth data is more recent
                    if last_price.time < pyth_price.publish_time {
                        let price: Price = pyth_price.into();
                        storage.price.write(price);
                        return price.value;
                    }
                } else {
                    // Redstone data is more recent
                    if last_price.time < redstone_timestamp {
                        let price = Price::new(redstone_price, redstone_timestamp);
                        storage.price.write(price);
                        return price.value;
                    }
                }
                // If both new prices are older than the last stored price, return the last price
                return last_price.value;
            }

            // Redstone data is fresh, update if it's newer than the last stored price
            if last_price.time < redstone_timestamp {
                let price = Price::new(redstone_price, redstone_timestamp);
                storage.price.write(price);
                return price.value;
            }

            // Otherwise, return the last stored price
            return last_price.value;
        }

        // Pyth data is fresh, update if it's newer than the last stored price
        if last_price.time < pyth_price.publish_time {
            let price: Price = pyth_price.into();
            storage.price.write(price);
            return price.value;
        }

        // If the new Pyth price is older than the last stored price, return the last price
        return last_price.value;
    }

    #[storage(write)]
    fn set_debug_timestamp(timestamp: u64) {
        // Allow setting a custom timestamp for testing, but only in debug mode
        require(DEBUG, "ORACLE: Debug is not enabled");
        storage.debug_timestamp.write(timestamp);
    }
}
