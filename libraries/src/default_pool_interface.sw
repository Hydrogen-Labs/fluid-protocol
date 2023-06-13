library default_pool_interface;

abi DefaultPool {
    #[storage(read, write)]
    fn initialize(trove_manager: Identity, active_pool: ContractId);

    #[storage(read, write)]
    fn send_asset_to_active_pool(amount: u64, asset_id: ContractId);

    #[storage(read)]
    fn get_asset(asset_id: ContractId) -> u64;

    #[storage(read, write)]
    fn add_asset(asset: ContractId, trove_manager: Identity);

    #[storage(read)]
    fn get_usdf_debt(asset_id: ContractId) -> u64;

    #[storage(read, write)]
    fn increase_usdf_debt(amount: u64, asset_id: ContractId);

    #[storage(read, write)]
    fn decrease_usdf_debt(amount: u64, asset_id: ContractId);

    #[storage(read, write), payable]
    fn recieve();
}
