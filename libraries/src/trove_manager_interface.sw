library trove_manager_interface;

dep data_structures;
use data_structures::{Status};

abi TroveManager {
    #[storage(read)]
    fn get_nominal_icr(id: Identity) -> u64;

    #[storage(read, write)]
    fn initialize(borrow_operations: ContractId, sorted_troves: ContractId);

    #[storage(read, write)]
    fn remove_stake(id: Identity);

    #[storage(read, write)]
    fn close_trove(id: Identity);

    #[storage(read, write)]
    fn set_nominal_icr(id: Identity, value: u64);

    #[storage(read, write)]
    fn set_trove_status(id: Identity, value: Status);

    #[storage(read, write)]
    fn increase_trove_coll(id: Identity, value: u64) -> u64;

    #[storage(read, write)]
    fn increase_trove_debt(id: Identity, value: u64) -> u64;

    #[storage(read, write)]
    fn decrease_trove_coll(id: Identity, value: u64) -> u64;

    #[storage(read, write)]
    fn decrease_trove_debt(id: Identity, value: u64) -> u64;

    #[storage(read, write)]
    fn add_trove_owner_to_array(id: Identity) -> u64;

    #[storage(read, write)]
    fn remove(id: Identity);

    #[storage(read)]
    fn get_trove_debt(id: Identity) -> u64;

    #[storage(read)]
    fn get_trove_coll(id: Identity) -> u64;

    #[storage(read, write)]
    fn set_nominal_icr_and_insert(id: Identity, value: u64, prev_id: Identity, next_id: Identity);
}
