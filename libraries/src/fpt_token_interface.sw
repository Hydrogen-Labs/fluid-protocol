library;

use std::string::String;

abi FPTToken {
    // Initialize contract
    #[storage(read, write)]
    fn initialize(
        vesting_contract: ContractId,
        community_issuance_contract: ContractId,
    );

    #[storage(read)]
    fn get_vesting_contract() -> ContractId;
}
