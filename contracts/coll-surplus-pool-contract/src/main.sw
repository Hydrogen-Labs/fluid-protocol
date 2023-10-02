contract;

use libraries::coll_surplus_pool_interface::CollSurplusPool;
use libraries::fluid_math::{null_contract, null_identity_address};

use std::{
    auth::msg_sender,
    call_frames::{
        msg_asset_id,
    },
    context::{
        msg_amount,
    },
    logging::log,
    token::transfer,
};

storage {
    protocol_manager: Identity = null_identity_address(),
    borrow_operations_contract: ContractId = null_contract(),
    aswith_amount: StorageMap<ContractId, u64> = StorageMap {},
    balances: StorageMap<(Identity, ContractId), u64> = StorageMap {},
    valid_asset_ids: StorageMap<ContractId, bool> = StorageMap {},
    valid_trove_managers: StorageMap<Identity, bool> = StorageMap {},
    is_initialized: bool = false,
}

impl CollSurplusPool for Contract {
    #[storage(read, write)]
    fn initialize(
        borrow_operations_contract: ContractId,
        protocol_manager: Identity,
    ) {
        require(storage.is_initialized == false, "Contract is already initialized");

        storage.borrow_operations_contract = borrow_operations_contract;
        storage.protocol_manager = protocol_manager;
        storage.is_initialized = true;
    }

    #[storage(read, write)]
    fn add_asset(asset: ContractId, trove_manager: Identity) {
        require_is_protocol_manager();
        storage.valid_asset_ids.insert(asset, true);
        storage.valid_trove_managers.insert(trove_manager, true);
        storage.aswith_amount.insert(asset, 0);
    }

    #[storage(read, write)]
    fn claim_coll(account: Identity, asset: ContractId) {
        require_is_borrow_operations_contract();
        require_is_valid_asset_id(asset);

        let balance = storage.balances.get((account, asset));
        if balance > 0 {
            storage.balances.insert((account, asset), 0);
            let aswith_amount = storage.aswith_amount.get(asset);
            storage.aswith_amount.insert(asset, aswith_amount - balance);

            transfer(balance, asset, account);
        }
    }

    #[storage(read, write)]
    fn account_surplus(account: Identity, amount: u64, asset: ContractId) {
        require_is_trove_manager();
        require_is_valid_asset_id(asset);

        let current_aswith_amount = storage.aswith_amount.get(asset);
        storage.aswith_amount.insert(asset, current_aswith_amount + amount);

        let mut balance = storage.balances.get((account, asset));
        balance += amount;
        storage.balances.insert((account, asset), balance);
    }

    #[storage(read)]
    fn get_asset(asset: ContractId) -> u64 {
        storage.aswith_amount.get(asset)
    }

    #[storage(read)]
    fn get_collateral(acount: Identity, asset: ContractId) -> u64 {
        storage.balances.get((acount, asset))
    }
}

#[storage(read)]
fn require_is_valid_asset_id(contract_id: ContractId) {
    let is_valid = storage.valid_asset_ids.get(contract_id);
    require(is_valid, "CSP: Invalid asset");
}

#[storage(read)]
fn require_is_protocol_manager() {
    let caller = msg_sender().unwrap();
    let protocol_manager = storage.protocol_manager;
    require(caller == protocol_manager, "CSP: Caller is not PM");
}

#[storage(read)]
fn require_is_trove_manager() {
    let caller = msg_sender().unwrap();
    let is_valid = storage.valid_trove_managers.get(caller);
    require(is_valid, "CSP: Caller is not TM");
}

#[storage(read)]
fn require_is_borrow_operations_contract() {
    let caller = msg_sender().unwrap();
    let borrow_operations_contract = Identity::ContractId(storage.borrow_operations_contract);
    require(caller == borrow_operations_contract, "CSP: Caller is not BO");
}
