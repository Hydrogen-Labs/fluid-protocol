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
// 
// IMPORTANT: The price returned by this contract should be the price in USD for 1_000_000_000 units of the asset in the Fuel network.
// Example 1: if the price of ETH is $3,000 USD with a 9 decimal representation in the fuel network, then the price of ETH is 3_000_000_000_000.
// Example 2: the price of BTC is $60,000 USD with an 8 decimal representation in the fuel network, then the price of 1 BTC is 60_000_000_000_000.
// But 1_000_000_000 units of BTC in the fuelvm with 8 decimal precision is 10 BTC, so the price returned by this contract should be 600_000_000_000_000 for 10 BTC.
// Refer to https://github.com/FuelLabs/verified-assets/blob/main/assets.json for the number of units that 1_000_000_000 units of an asset is in the fuelvm.

use libraries::{
    fluid_math::{
        convert_precision,
        convert_precision_u256_and_downcast,
    },
    oracle_interface::RedstoneCore,
    oracle_interface::{
        Oracle,
        RedstoneConfig,
    },
};
use std::{block::timestamp, constants::ZERO_B256,};
use pyth_interface::{data_structures::price::{Price, PriceFeedId}, PythCore};

// // Hack: Sway does not provide a downcast to u64
// // If redstone provides a strangely high u256 which shouldn't be cast down
// // then other parts of the code must be adjusted to use u256

configurable {
    /// Contract Address
    PYTH: ContractId = ContractId::zero(),
    /// Price feed to query
    PYTH_PRICE_ID: PriceFeedId = ZERO_B256,
    /// Decimal representation of the asset in the Fuel network
    FUEL_DECIMAL_REPRESENTATION: u32 = 9,
    /// Timeout in seconds
    DEBUG: bool = false,
    /// Initializer
    INITIALIZER: Identity = Identity::Address(Address::zero()),
}
// Timeout period for considering oracle data as stale (10 minutes in seconds)
const TIMEOUT: u64 = 600;

const TAI64_UNIX_ADJUSTMENT = (10 + (1 << 62));
// https://github.com/redstone-finance/redstone-oracles-monorepo/blob/ba7af63c0e3f09fa7aecd7dc4eedd4f1d4664083/packages/fuel-connector/sway/common/src/timestamp.sw#L5

storage {
    /// The last valid price from either Pyth or Redstone
    last_good_price: Price = Price {
        confidence: 0,
        exponent: 0,
        price: 0,
        publish_time: 0,
    },
    // Used for simulating different timestamps during testing
    debug_timestamp: u64 = 0,
    redstone_config: Option<RedstoneConfig> = None,
}

impl Oracle for Contract {
    #[storage(read, write)]
    fn get_price() -> u64 {
        // Step 1: Query the Pyth oracle (primary source)
        let mut pyth_price = abi(PythCore, PYTH.bits()).price_unsafe(PYTH_PRICE_ID);
        pyth_price = pyth_price_with_fuel_vm_precision_adjustment(pyth_price, FUEL_DECIMAL_REPRESENTATION);
        let redstone_config = storage.redstone_config.read();
        // If Redstone is not configured, return the latest Pyth price regardless of staleness and confidence
        if redstone_config.is_none() {
            return pyth_price.price;
        }
        // Determine the current timestamp based on debug mode
        let current_time = match DEBUG {
            true => storage.debug_timestamp.read(),
            false => timestamp(),
        };
        // Read the last stored valid price
        let last_price = storage.last_good_price.read();
        // Check if Pyth data is stale or outside confidence
        if is_pyth_price_stale_or_outside_confidence(pyth_price, current_time) {
            // Step 2: Pyth is stale or outside confidence, query Redstone oracle (fallback source)
            let config = redstone_config.unwrap();

            let mut feed = Vec::with_capacity(1);
            feed.push(config.price_id);

            // Fuel Bug workaround: trait coherence
            let id = config.contract_id.bits();
            let redstone = abi(RedstoneCore, id);
            let redstone_prices = redstone.read_prices(feed);
            let redstone_timestamp = redstone.read_timestamp() + TAI64_UNIX_ADJUSTMENT; // Redstone uses unix where we use tai64 in fuel
            let redstone_price_u64 = redstone_prices.get(0).unwrap();
            // By default redstone uses 8 decimal precision so it is generally safe to cast down
            let redstone_price = convert_precision_u256_and_downcast(
                redstone_price_u64,
                adjust_exponent(config.precision, FUEL_DECIMAL_REPRESENTATION),
            );
            // Check if Redstone data is also stale
            if current_time > redstone_timestamp + TIMEOUT {
                // Both oracles are stale, use the most recent data available
                if redstone_timestamp <= pyth_price.publish_time {
                    // Pyth data is more recent
                    if last_price.publish_time < pyth_price.publish_time {
                        let price: Price = pyth_price;
                        storage.last_good_price.write(price);
                        return price.price;
                    }
                } else {
                    // Redstone data is more recent
                    if last_price.publish_time < redstone_timestamp {
                        let price = Price::new(0, 0, redstone_price, redstone_timestamp);
                        storage.last_good_price.write(price);
                        return price.price;
                    }
                }
                // If both new prices are older than the last stored price, return the last price
                return last_price.price;
            }

            // Redstone data is fresh, update if it's newer than the last stored price
            if last_price.publish_time < redstone_timestamp {
                let price = Price::new(0, 0, redstone_price, redstone_timestamp);
                storage.last_good_price.write(price);
                return price.price;
            }

            // Otherwise, return the last stored price
            return last_price.price;
        }
        // Pyth data is fresh, update if it's newer than the last stored price
        if last_price.publish_time < pyth_price.publish_time {
            let price: Price = pyth_price;
            storage.last_good_price.write(price);
            return price.price;
        }

        // If the new Pyth price is older than the last stored price, return the last price
        return last_price.price;
    }

    #[storage(write)]
    fn set_debug_timestamp(timestamp: u64) {
        // Allow setting a custom timestamp for testing, but only in debug mode
        require(DEBUG, "ORACLE: Debug is not enabled");
        storage.debug_timestamp.write(timestamp);
    }

    #[storage(read, write)]
    fn set_redstone_config(config: RedstoneConfig) {
        require(
            msg_sender()
                .unwrap() == INITIALIZER,
            "ORACLE: Only initializer can set Redstone config",
        );
        require(
            storage
                .redstone_config
                .read()
                .is_none(),
            "ORACLE: Redstone config already set",
        );
        storage.redstone_config.write(Some(config));
    }
}
// Assets in the fuel VM can have a different decimal representation
// This function adjusts the price to align with the decimal representation of the Fuel VM
fn pyth_price_with_fuel_vm_precision_adjustment(pyth_price: Price, fuel_vm_decimals: u32) -> Price {
    let adjusted_exponent = adjust_exponent(pyth_price.exponent, fuel_vm_decimals);
    return Price {
        confidence: convert_precision(pyth_price.confidence, adjusted_exponent),
        price: convert_precision(pyth_price.price, adjusted_exponent),
        publish_time: pyth_price.publish_time,
        exponent: adjusted_exponent,
    };
}

fn adjust_exponent(current_exponent: u32, fuel_vm_decimals: u32) -> u32 {
    if fuel_vm_decimals > 9u32 {
        // If the Fuel VM has more decimals than 9 we need to remove the extra precision
        // For example, if the Fuel VM has 10 decimals then we need to divide the price by 10^1 to get the correct price for 1_000_000_000 units
        return current_exponent + (fuel_vm_decimals - 9u32);
    } else if fuel_vm_decimals < 9u32 {
        // If the Fuel VM has less decimals than 9 we need to add the missing precision
        // Smaller precision means we need to add more precision to the price
        return current_exponent - (9u32 - fuel_vm_decimals);
    }
    return current_exponent;
}

fn is_pyth_price_stale_or_outside_confidence(pyth_price: Price, current_time: u64) -> bool {
    // confidence within 4% is considered safe 
    let confidence_threshold = pyth_price.price / 25;
    return current_time > pyth_price.publish_time + TIMEOUT || pyth_price.confidence > confidence_threshold;
}

#[test]
fn test_is_pyth_price_not_stale_or_outside_confidence() {
    let pyth_price = Price {
        confidence: 2,
        publish_time: 100,
        price: 100,
        exponent: 0,
    };
    let is_stale_or_outside_confidence = is_pyth_price_stale_or_outside_confidence(pyth_price, 100);
    assert(is_stale_or_outside_confidence == false);
}

#[test]
fn test_is_price_outside_confidence() {
    // confidence is 5, which is outside 5% threshold 
    let pyth_price = Price {
        confidence: 5,
        publish_time: 100,
        price: 100,
        exponent: 0,
    };
    let is_stale_or_outside_confidence = is_pyth_price_stale_or_outside_confidence(pyth_price, 100);
    assert(is_stale_or_outside_confidence == true);
}

#[test]
fn test_is_price_stale() {
    // price is stale because it's published more than 4 hours ago
    let publish_time = 10000;
    let pyth_price = Price {
        confidence: 2,
        publish_time: publish_time,
        price: 100,
        exponent: 0,
    };
    let is_stale_or_outside_confidence = is_pyth_price_stale_or_outside_confidence(pyth_price, publish_time + TIMEOUT + 1);
    assert(is_stale_or_outside_confidence == true);
}

#[test]
fn test_pyth_price_adjustment_fuel_vm_decimals_equal() {
    // Fuel VM has 9 decimals
    let fuel_vm_decimals = 9;
    let original_price = Price {
        confidence: 100,
        price: 1_000_000_000,
        publish_time: 1000,
        exponent: 9,
    };
    let adjusted_price = pyth_price_with_fuel_vm_precision_adjustment(original_price, fuel_vm_decimals);
    assert(adjusted_price.price == 1_000_000_000);
    assert(adjusted_price.confidence == 100);
    assert(adjusted_price.exponent == 9);
    assert(adjusted_price.publish_time == 1000);
}

#[test]
fn test_pyth_price_adjustment_fuel_vm_decimals_greater_than_9() {
    // Fuel VM has 10 decimals
    let fuel_vm_decimals = 10;
    // With 10 decimals, the price of 1_000_000_000 units of an asset is 100_000_000
    let original_price = Price {
        confidence: 100,
        price: 1_000_000_000,
        publish_time: 1000,
        exponent: 9,
    };
    let adjusted_price = pyth_price_with_fuel_vm_precision_adjustment(original_price, fuel_vm_decimals);
    assert(adjusted_price.price == 100_000_000);
    assert(adjusted_price.confidence == 10);
    assert(adjusted_price.exponent == 10);
    assert(adjusted_price.publish_time == 1000);
}

#[test]
fn test_pyth_price_adjustment_fuel_vm_decimals_less_than_9() {
    // Fuel VM has 8 decimals
    let fuel_vm_decimals = 8;
    // With 8 decimals, the price of 1_000_000_000 units of an asset is 10_000_000_000
    let original_price = Price {
        confidence: 100,
        price: 1_000_000_000,
        publish_time: 1000,
        exponent: 9,
    };
    let adjusted_price = pyth_price_with_fuel_vm_precision_adjustment(original_price, fuel_vm_decimals);
    assert(adjusted_price.price == 10_000_000_000);
    assert(adjusted_price.confidence == 1000);
    assert(adjusted_price.exponent == 8);
    assert(adjusted_price.publish_time == 1000);
}

#[test]
fn test_pyth_price_adjustment_fuel_vm_decimals_8_initial_exponent_8() {
    // Fuel VM has 8 decimals
    let fuel_vm_decimals = 8; // add 10^1 to the price and confidence
    // Initial Pyth price with exponent 8
    let original_price = Price {
        confidence: 1000,
        price: 1_000_000_000,
        publish_time: 1000,
        exponent: 8, // add a second 10^1 to the price and confidence
    };
    let adjusted_price = pyth_price_with_fuel_vm_precision_adjustment(original_price, fuel_vm_decimals);
    // Expected: price for 1_000_000_000 units (10 units with 8 decimals) should be 1_000_000_000 ($10.00)
    assert(adjusted_price.price == 100_000_000_000);
    assert(adjusted_price.confidence == 100_000);
    assert(adjusted_price.exponent == 7);
    assert(adjusted_price.publish_time == 1000);
}
