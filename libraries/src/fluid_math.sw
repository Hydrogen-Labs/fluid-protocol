library;

pub mod numbers;
use numbers::*;
use std::{hash::*, u128::U128};

pub const ZERO_B256 = 0x0000000000000000000000000000000000000000000000000000000000000000;

pub const SECONDS_IN_ONE_MINUTE: u64 = 60;

pub const DECIMAL_PRECISION: u64 = 1_000_000_000;

// Redemption fee floor is 1%
pub const REDEMPTION_FEE_FLOOR: u64 = 10_000_000;

// Min borrowing fee is 0.5%
pub const BORROWING_FEE_FLOOR: u64 = 5_000_000;

pub const MCR: u64 = 1_350_000_000;
// 10 USDF 
pub const USDF_GAS_COMPENSATION: u64 = 10_000_000;

// min debt is 500 USDF for staging 
pub const MIN_NET_DEBT: u64 = 500_000_000_000; /*  */ pub const PERCENT_DIVERSOR = 200;

pub const POST_COLLATERAL_RATIO: u64 = 1_500_000_000;

// 10% fee
pub const STABILITY_POOL_FEE: u64 = 100_000_000;

// 0.5% fee going to person who liquidates
pub const LIQUIDATOR_EXECUTION_GAS_FEE: u64 = 5_000_000;

pub const ONE: u64 = 1_000_000_000;
pub const BETA: u64 = 2;

pub fn convert_precision(price: u64, current_precision: u32) -> u64 {
    let mut adjusted_price = 0;
    if current_precision > 9 {
        let precision = current_precision - 9;
        let magnitude = 10.pow(precision);
        adjusted_price = price / magnitude;
    } else if current_precision < 9 {
        let precision = 9_u32 - current_precision;
        let magnitude = 10.pow(precision);
        adjusted_price = price * magnitude;
    } else {
        adjusted_price = price;
    }

    adjusted_price
}

// Convert precision first to ensure accuracy, then downcast to u64 to fit Fluid's standard format
pub fn convert_precision_u256_and_downcast(price: u256, current_precision: u32) -> u64 {
    let mut adjusted_price = 0;
    if current_precision > 9 {
        let precision = current_precision - 9;
        let magnitude = 10.pow(precision).into();
        adjusted_price = price / magnitude;
    } else if current_precision < 9 {
        let precision = 9_u32 - current_precision;
        let magnitude = 10.pow(precision).into();
        adjusted_price = price * magnitude;
    } else {
        adjusted_price = price;
    }
    u64::try_from(adjusted_price).unwrap()
}

/// Saturating U128 -> u64 conversion, used ONLY by the collateral-ratio
/// functions below.
///
/// `fm_compute_cr` / `fm_compute_nominal_cr` produce a RATIO that is only ever
/// compared (`require_at_least_mcr`, `icr < MCR`) or used as a sorted-list key.
/// For those, "does not fit in u64" means "extremely over-collateralised", so
/// saturating to `u64::max()` is semantically correct, and it matches the
/// existing `debt == 0` branch which already returns `u64::max()`.
///
/// Do NOT use this for AMOUNTS (`fm_multiply_ratio`, the fee functions): those
/// feed real token quantities. Silently returning `u64::max()` there would be
/// worse than reverting, so they keep `.as_u64().unwrap()` and fail loudly.
pub fn fm_saturating_u64(value: U128) -> u64 {
    match value.as_u64() {
        Ok(v) => v,
        Err(_) => u64::max(),
    }
}

// 0.5% one-time borrow fee
pub fn fm_compute_borrow_fee(debt: u64) -> u64 {
    let fee = U128::from(debt) * U128::from(BORROWING_FEE_FLOOR) / U128::from(DECIMAL_PRECISION);
    return fee.as_u64().unwrap();
}

// 1% redemption fee
pub fn fm_compute_redemption_fee(debt: u64) -> u64 {
    let fee = U128::from(debt) * U128::from(REDEMPTION_FEE_FLOOR) / U128::from(DECIMAL_PRECISION);
    return fee.as_u64().unwrap();
}

pub fn fm_compute_nominal_cr(coll: u64, debt: u64) -> u64 {
    if (debt > 0) {
        let ncr: U128 = U128::from(coll) * U128::from(DECIMAL_PRECISION) / U128::from(debt);
        return fm_saturating_u64(ncr);
    } else {
        return u64::max();
    }
}

pub fn fm_multiply_ratio(value: u64, numerator: u64, denominator: u64) -> u64 {
    let ratio: U128 = U128::from(value) * U128::from(numerator) / U128::from(denominator);
    return ratio.as_u64().unwrap();
}

pub fn fm_compute_cr(coll: u64, debt: u64, price: u64) -> u64 {
    if (debt > 0) {
        let cr: U128 = U128::from(coll) * U128::from(price) / U128::from(debt);
        return fm_saturating_u64(cr);
    } else {
        return u64::max();
    }
}

pub fn fm_abs_diff(a: u64, b: u64) -> u64 {
    if a > b { return a - b; } else { return b - a; }
}

pub fn assert_within_percent_tolerance(a: u64, b: u64, tolerance: u64) {
    let diff = fm_abs_diff(a, b);
    let max_diff = fm_min(a, b) * tolerance / 1_000_000_000;
    assert(diff <= max_diff);
}

pub fn fm_min(a: u64, b: u64) -> u64 {
    if a < b { return a; } else { return b; }
}

pub fn fm_min_u128(a: U128, b: U128) -> U128 {
    if a < b { return a; } else { return b; }
}

pub fn fm_max(a: u64, b: u64) -> u64 {
    if a > b { return a; } else { return b; }
}

pub fn dec_mul(a: U128, b: U128) -> U128 {
    let prod = a * b;
    let dec_prod = (prod + U128::from(DECIMAL_PRECISION / 2)) / U128::from(DECIMAL_PRECISION);
    return dec_prod;
}

pub fn dec_pow(base: u64, _minutes: u64) -> U128 {
    let mut minutes = _minutes;
    if minutes > 525600000 {
        minutes = 525600000;
    }

    let mut y = U128::from(DECIMAL_PRECISION);
    let mut x = U128::from(base);
    let mut n = U128::from(minutes);

    while n > U128::from(1u64) {
        if n % U128::from(2u64) == U128::from(0u64) {
            x = dec_mul(x, x);
            n = n / U128::from(2u64);
        } else {
            y = dec_mul(x, y);
            x = dec_mul(x, x);
            n = (n - U128::from(1u64)) / U128::from(2u64);
        }
    }

    return dec_mul(x, y);
}

pub fn null_identity_address() -> Identity {
    return Identity::Address(Address::zero())
}

pub fn null_contract() -> ContractId {
    return ContractId::zero()
}

#[test]
fn test_dec_pow_zero() {
    let base = 1_000_000_000;
    let exponent = 0;
    let result = dec_pow(base, exponent);
    assert(result == U128::from(DECIMAL_PRECISION));
}

#[test]
fn test_dec_pow_one() {
    let base = 1_000_000_000;
    let exponent = 1;
    let result = dec_pow(base, exponent);
    assert(base == result.as_u64().unwrap());

    let base = 3_000_000_000;
    let exponent = 1;
    let result = dec_pow(base, exponent);
    assert(base == result.as_u64().unwrap());
}

#[test]
fn test_dec_pow_two() {
    let base = 1_500_000_000;
    let exponent = 2;
    let result = dec_pow(base, exponent);
    assert(2_250_000_000 == result.as_u64().unwrap());

    let base = 3_000_000_000;
    let exponent = 2;
    let result = dec_pow(base, exponent);
    assert(9_000_000_000 == result.as_u64().unwrap());
}

#[test]
fn test_precision_less_than_current() {
    let price = 1_000_000_000_000;
    let precision = 8;
    let result = convert_precision(price, precision);
    assert_eq(result, price * 10);
}

#[test]
fn test_precision_more_than_current() {
    let price = 1_000_000_000_000;
    let precision = 10;
    let result = convert_precision(price, precision);
    assert_eq(result, price / 10);
}

#[test]
fn test_precision_is_equal_to_current() {
    let price = 1_000_000_000_000;
    let precision = 9;
    let result = convert_precision(price, precision);
    assert_eq(result, price);
}

#[test]
fn test_precision_less_than_current_pow() {
    let price = 1_000_000_000_000;
    let precision = 6;

    let result = convert_precision(price, precision);
    assert_eq(result, price * 10.pow(3));
}

#[test]
fn test_precision_u256_less_than_current() {
    let price: u256 = 1_000_000_000_000;
    let precision = 8;
    let result = convert_precision_u256_and_downcast(price, precision);
    assert_eq(result, 10_000_000_000_000);
}

#[test]
fn test_precision_u256_more_than_current() {
    let price: u256 = 1_000_000_000_000;
    let precision = 10;
    let result = convert_precision_u256_and_downcast(price, precision);
    assert_eq(result, 100_000_000_000);
}

#[test]
fn test_precision_u256_is_equal_to_current() {
    let price: u256 = 1_000_000_000_000;
    let precision = 9;
    let result = convert_precision_u256_and_downcast(price, precision);
    assert_eq(result, 1_000_000_000_000);
}

#[test]
fn test_precision_u256_less_than_current_pow() {
    let price: u256 = 1_000_000;
    let precision = 6;
    let result = convert_precision_u256_and_downcast(price, precision);
    assert_eq(result, 1_000_000_000);
}

// ---------------------------------------------------------------------------
// Regression tests: a CR above u64::MAX must saturate, not revert.
//
// Liquity computes CR in uint256, so a huge CR is harmless. This port computes
// in U128 and previously did `.as_u64().unwrap()`, which PANICKED (tx revert)
// when coll*price/debt exceeded u64::MAX (18_446_744_073_709_551_615). With
// fm_saturating_u64 it now returns u64::max(), matching the existing debt == 0
// branch.
// ---------------------------------------------------------------------------
#[test]
fn test_cr_above_u64_max_saturates() {
    // coll = 1e18, price = 1e13, debt = 1  ->  CR = 1e31 (>> u64::MAX)
    let cr = fm_compute_cr(1_000_000_000_000_000_000, 1, 10_000_000_000_000);
    assert_eq(cr, u64::max());
}

#[test]
fn test_nominal_cr_above_u64_max_saturates() {
    let ncr = fm_compute_nominal_cr(1_000_000_000_000_000_000, 1);
    assert_eq(ncr, u64::max());
}

#[test]
fn test_cr_at_realistic_values_is_unchanged() {
    // 1.5x overcollateralised: CR must come back as ~1.5e9 (150%).
    let cr = fm_compute_cr(3_000_000_000, 2_000_000_000, 1_000_000_000);
    assert_eq(cr, 1_500_000_000);
}
