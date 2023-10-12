contract;

use libraries::coll_surplus_pool_interface::CollSurplusPool;
use libraries::fluid_math::ZERO_B256;
use std::{
    auth::msg_sender,
    call_frames::{
        msg_asset_id,
    },
    context::{
        msg_amount,
    },
    hash::Hash,
    logging::log,
    token::transfer,
};
storage {
    protocol_manager: Identity = Identity::Address(Address::from(ZERO_B256)),
    borrow_operations_contract: ContractId = ContractId::from(ZERO_B256),
    asset_amount: StorageMap<AssetId, u64> = StorageMap::<AssetId, u64> {},
    balances: StorageMap<(Identity, AssetId), u64> = StorageMap::<(Identity, AssetId), u64> {},
    valid_asset_ids: StorageMap<AssetId, bool> = StorageMap::<AssetId, bool> {},
    valid_trove_managers: StorageMap<Identity, bool> = StorageMap::<Identity, bool> {},
    is_initialized: bool = false,
}
impl CollSurplusPool for Contract {
    #[storage(read, write)]
    fn initialize(
        borrow_operations_contract: ContractId,
        protocol_manager: Identity,
    ) {
        require(storage.is_initialized.read() == false, "CSP: Contract is already initialized");
        storage.borrow_operations_contract.write(borrow_operations_contract);
        storage.protocol_manager.write(protocol_manager);
        storage.is_initialized.write(true);
    }
    #[storage(read, write)]
    fn add_asset(asset: AssetId, trove_manager: Identity) {
        require_is_protocol_manager();
        storage.valid_asset_ids.insert(asset, true);
        storage.valid_trove_managers.insert(trove_manager, true);
        storage.asset_amount.insert(asset, 0);
    }
    #[storage(read, write)]
    fn claim_coll(account: Identity, asset: AssetId) {
        require_is_borrow_operations_contract();
        require_is_valid_asset_id(asset);
        let balance = storage.balances.get((account, asset)).try_read().unwrap_or(0);

        if balance > 0 {
            storage.balances.insert((account, asset), 0);
            let asset_amount = storage.asset_amount.get(asset).read();
            storage.asset_amount.insert(asset, asset_amount - balance);
            transfer(account, asset, balance);
        }
    }
    #[storage(read, write)]
    fn account_surplus(account: Identity, amount: u64, asset: AssetId) {
        require_is_trove_manager();
        require_is_valid_asset_id(asset);
        let current_asset_amount = storage.asset_amount.get(asset).try_read().unwrap_or(0);
        storage.asset_amount.insert(asset, current_asset_amount + amount);
        let mut balance = storage.balances.get((account, asset)).try_read().unwrap_or(0);
        balance += amount;
        storage.balances.insert((account, asset), balance);
    }
    #[storage(read)]
    fn get_asset(asset: AssetId) -> u64 {
        return storage.asset_amount.get(asset).try_read().unwrap_or(0)
    }
    #[storage(read)]
    fn get_collateral(acount: Identity, asset: AssetId) -> u64 {
        return storage.balances.get((acount, asset)).try_read().unwrap_or(0)
    }
}
#[storage(read)]
fn require_is_valid_asset_id(contract_id: AssetId) {
    let is_valid = storage.valid_asset_ids.get(contract_id).try_read().unwrap_or(false);
    require(is_valid, "CSP: Invalid asset");
}
#[storage(read)]
fn require_is_protocol_manager() {
    let caller = msg_sender().unwrap();
    let protocol_manager = storage.protocol_manager.read();
    require(caller == protocol_manager, "CSP: Caller is not PM");
}
#[storage(read)]
fn require_is_trove_manager() {
    let caller = msg_sender().unwrap();
    let is_valid = storage.valid_trove_managers.get(caller).read();
    require(is_valid, "CSP: Caller is not TM");
}
#[storage(read)]
fn require_is_borrow_operations_contract() {
    let caller = msg_sender().unwrap();
    let borrow_operations_contract = Identity::ContractId(storage.borrow_operations_contract.read());
    require(caller == borrow_operations_contract, "CSP: Caller is not BO");
}
