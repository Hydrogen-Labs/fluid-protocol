library;

use std::string::String;

abi FPTToken {
    // Initialize contract
    #[storage(read, write)]
    fn initialize(
        vesting_contract: ContractId,
        community_issuance_contract: ContractId,
        debugging: bool,
    );

    #[storage(read)]
    fn get_vesting_contract() -> ContractId;

    fn total_assets() -> u64;

    #[storage(read)]
    fn total_supply(asset: AssetId) -> Option<u64>;

    #[storage(read)]
    fn name(asset: AssetId) -> Option<String>;

    #[storage(read)]
    fn symbol(asset: AssetId) -> Option<String>;

    #[storage(read)]
    fn decimals(asset: AssetId) -> Option<u8>;

    #[storage(read)]
    fn mint_to_id(amount: u64, address: Identity);
}
