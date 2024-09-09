contract;
// This contract, FPTToken, implements the Fluid Protocol Token (FPT) functionality.
// FPT is the native token of the Fluid Protocol
//
// Key functionalities include:
// - Minting and distributing the initial supply of FPT tokens
// - Managing token transfers and approvals
// - Interfacing with the Vesting and Community Issuance contracts
//
// The contract follows the SRC-20 standard for native assets on the Fuel network,
// ensuring compatibility with the broader ecosystem. It manages a fixed total supply
// of 100 million FPT tokens, distributed between the Vesting contract and the
// Community Issuance contract upon initialization.

use libraries::fpt_token_interface::FPTToken;
use libraries::fluid_math::{DECIMAL_PRECISION, get_default_asset_id, ZERO_B256,};
use std::{
    address::*,
    asset::*,
    auth::{
        AuthError,
    },
    call_frames::{
        msg_asset_id,
    },
    context::{
        balance_of,
        msg_amount,
    },
    storage::*,
    string::String,
};

storage {
    vesting_contract: ContractId = ContractId::zero(),
    community_issuance_contract: ContractId = ContractId::zero(),
    is_initialized: bool = false,
    default_asset: AssetId = AssetId::zero(),
    debug: bool = false,
}
// Using https://docs.fuel.network/docs/sway-standards/src-20-native-asset/ as reference
// import fluid math decimals here
pub const TOTAL_SUPPLY: u64 = 100_000_000;
impl FPTToken for Contract {
    //////////////////////////////////////
    // Initialization method
    //////////////////////////////////////
    #[storage(read, write)]
    fn initialize(
        vesting_contract: ContractId,
        community_issuance_contract: ContractId,
        debugging: bool,
    ) {
        require(
            storage
                .is_initialized
                .read() == false,
            "FPTToken: Contract is already initialized",
        );
        storage.vesting_contract.write(vesting_contract);
        storage
            .community_issuance_contract
            .write(community_issuance_contract);
        mint_to(
            Identity::ContractId(vesting_contract),
            ZERO_B256,
            TOTAL_SUPPLY * 68 / 100 * DECIMAL_PRECISION,
        );
        mint_to(
            Identity::ContractId(community_issuance_contract),
            ZERO_B256,
            TOTAL_SUPPLY * 32 / 100 * DECIMAL_PRECISION,
        );
        storage
            .default_asset
            .write(get_default_asset_id(ContractId::this()));
        storage.is_initialized.write(true);
        storage.debug.write(debugging);
    }
    //////////////////////////////////////
    // Read-Only methods
    //////////////////////////////////////
    #[storage(read)]
    fn get_vesting_contract() -> ContractId {
        storage.vesting_contract.read()
    }
    //////////////////////////////////////
    // SRC-20 Read-Only methods
    //////////////////////////////////////
    fn total_assets() -> u64 {
        return 1;
    }

    #[storage(read)]
    fn total_supply(asset: AssetId) -> Option<u64> {
        if asset == storage.default_asset.read() {
            return Some(TOTAL_SUPPLY * DECIMAL_PRECISION);
        }
        return None;
    }

    #[storage(read)]
    fn name(asset: AssetId) -> Option<String> {
        if asset == storage.default_asset.read() {
            return Some(String::from_ascii_str("Fluid Protocol Token"));
        }
        return None;
    }

    #[storage(read)]
    fn symbol(asset: AssetId) -> Option<String> {
        if asset == storage.default_asset.read() {
            return Some(String::from_ascii_str("FPT"));
        }
        return None;
    }

    #[storage(read)]
    fn decimals(asset: AssetId) -> Option<u8> {
        if asset == storage.default_asset.read() {
            return Some(9u8);
        }
        return None;
    }

    /// @notice Mints FPT tokens to a specified address
    /// @dev This function can only be called when debugging is enabled. It is included to simplify testing for FPT staking.
    /// @param amount The amount of FPT tokens to mint
    /// @param address The address to mint the tokens to
    /// @custom:throws "FPTToken: Debugging must be enabled to mint" if debugging is not enabled
    #[storage(read)]
    fn mint_to_id(amount: u64, address: Identity) {
        require(
            storage
                .debug
                .read(),
            "FPTToken: Debugging must be enabled to mint",
        );
        mint_to(address, ZERO_B256, amount);
    }
}
