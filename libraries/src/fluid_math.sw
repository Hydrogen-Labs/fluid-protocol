library fluid_math;

pub const PCT_100: u64 = 1_000_000_000;

pub const DECIMAL_PRECISION: u64 = 1_000_000_000;

pub const MCR: u64 = 120_000_000_000;

pub const MAX_U64: u64 = 18_446_744_073_709_551_615;
// 10 USDF 
pub const USDF_GAS_COMPENSATION: u64 = 10_000_000_000;

// min debt is 500 USDF
pub const MIN_NET_DEBT: u64 = 500_000_000_000;

pub const PERCENT_DIVERSOR = 200;

pub fn fm_get_net_debt(_debt: u64) -> u64 {
    return _debt + USDF_GAS_COMPENSATION;
}

pub fn fm_compute_nominal_cr(coll: u64, debt: u64) -> u64 {
    if (debt > 0) {
        return (coll * DECIMAL_PRECISION) / (debt);
    } else {
        return MAX_U64;
    }
}

pub fn fm_compute_cr(coll: u64, debt: u64, price: u64) -> u64 {
    if (debt > 0) {
        return (coll * price) / (debt);
    } else {
        return MAX_U64;
    }
}