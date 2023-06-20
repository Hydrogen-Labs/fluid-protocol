library protocol_manager_interface;

abi ProtocolManager {
    #[storage(read, write)]
    fn initialize(borrow_operations: ContractId, stability_pool: ContractId, fpt_staking: ContractId, usdf: ContractId, coll_surplus_pool: ContractId, default_pool: ContractId, active_pool: ContractId, sorted_troves: ContractId, admin: Identity);

    #[storage(read, write)]
    fn register_asset(asset_address: ContractId, trove_manager: ContractId, oracle: ContractId);

    #[storage(read, write)]
    fn renounce_admin();

    #[storage(read, write), payable]
    fn redeem_collateral(max_itterations: u64, max_fee_percentage: u64, partial_redemption_hint: u64, upper_partial_hint: Identity, lower_partial_hint: Identity);
}
