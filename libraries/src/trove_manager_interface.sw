library trove_manager_interface;

dep data_structures;
use data_structures::{Status, Trove};

abi TroveManager {
    #[storage(read, write)]
    fn initialize(borrow_operations: ContractId, sorted_troves: ContractId, oracle: ContractId, stability_pool: ContractId, default_pool: ContractId, active_pool: ContractId, coll_surplus_pool: ContractId, usdf_contract: ContractId);

    // TODO REMOVE 
    #[storage(read, write)]
    fn set_nominal_icr_and_insert(id: Identity, value: u64, prev_id: Identity, next_id: Identity);

    #[storage(read, write)]
    fn remove(id: Identity);

    #[storage(read)]
    fn get_nominal_icr(id: Identity) -> u64;

    #[storage(read)]
    fn get_current_icr(id: Identity, price: u64) -> u64;

    #[storage(read, write)]
    fn liquidate(id: Identity);

    #[storage(read, write)]
    fn liquidate_troves(num_troves: u64);

    #[storage(read, write)]
    fn batch_liquidate_troves(ids: Vec<Identity>);

    #[storage(read, write)]
    fn redeem_collateral(max_itterations: u64, max_fee_percentage: u64, partial_redemption_hint: u64, upper_partial_hint: Identity, lower_partial_hint: Identity);

    #[storage(read, write)]
    fn update_stake_and_total_stakes(id: Identity) -> u64;

    #[storage(read, write)]
    fn update_trove_reward_snapshots(id: Identity);

    #[storage(read, write)]
    fn add_trove_owner_to_array(id: Identity) -> u64;

    #[storage(read, write)]
    fn apply_pending_rewards(id: Identity);

    #[storage(read)]
    fn get_pending_asset_rewards(id: Identity) -> u64;

    #[storage(read)]
    fn get_pending_usdf_rewards(id: Identity) -> u64;

    #[storage(read)]
    fn has_pending_rewards(id: Identity) -> bool;

    #[storage(read)]
    fn get_entire_debt_and_coll(id: Identity) -> (u64, u64, u64, u64);

    #[storage(read, write)]
    fn close_trove(id: Identity);

    #[storage(read, write)]
    fn remove_stake(id: Identity);

    #[storage(read)]
    fn get_redemption_rate() -> u64;

    #[storage(read)]
    fn get_redemption_rate_with_decay() -> u64;

    #[storage(read)]
    fn get_borrowing_rate() -> u64;

    #[storage(read)]
    fn get_borrowing_rate_with_decay() -> u64;

    #[storage(read)]
    fn get_borrowing_fee(debt: u64) -> u64;

    #[storage(read)]
    fn get_borrowing_fee_with_decay(debt: u64) -> u64;

    #[storage(read, write)]
    fn decay_base_rate_from_borrowing();

    #[storage(read)]
    fn get_trove_status(id: Identity) -> Status;

    #[storage(read)]
    fn get_trove_stake(id: Identity) -> u64;

    #[storage(read)]
    fn get_trove_debt(id: Identity) -> u64;

    #[storage(read)]
    fn get_trove_coll(id: Identity) -> u64;

    #[storage(read, write)]
    fn set_trove_status(id: Identity, value: Status);

    #[storage(read, write)]
    fn increase_trove_coll(id: Identity, value: u64) -> u64;

    #[storage(read, write)]
    fn decrease_trove_coll(id: Identity, value: u64) -> u64;

    #[storage(read, write)]
    fn increase_trove_debt(id: Identity, value: u64) -> u64;

    #[storage(read, write)]
    fn decrease_trove_debt(id: Identity, value: u64) -> u64;

    #[storage(read)]
    fn get_tcr() -> u64;
}
