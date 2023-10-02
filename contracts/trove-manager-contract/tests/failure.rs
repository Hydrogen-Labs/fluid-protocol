use fuels::types::Identity;
use test_utils::{
    data_structures::PRECISION,
    interfaces::{
        borrow_operations::{borrow_operations_abi, BorrowOperations},
        oracle::oracle_abi,
        token::token_abi,
        trove_manager::trove_manager_abi,
    },
    setup::common::setup_protocol,
};

#[tokio::test]
async fn fails_to_liquidate_trove_not_under_mcr() {
    let (contracts, _admin, mut wallets) = setup_protocol(10, 5, false).await;

    oracle_abi::set_price(&contracts.aswith_contracts[0].oracle, 10 * PRECISION).await;

    let wallet1 = wallets.pop().unwrap();

    let balance = 25_000 * PRECISION;
    token_abi::mint_to_id(
        &contracts.aswith_contracts[0].asset,
        balance,
        Identity::Address(wallet1.address().into()),
    )
    .await;

    let borrow_operations_wallet1 = BorrowOperations::new(
        contracts.borrow_operations.contract_id().clone(),
        wallet1.clone(),
    );

    borrow_operations_abi::open_trove(
        &borrow_operations_wallet1,
        &contracts.aswith_contracts[0].oracle,
        &contracts.aswith_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.aswith_contracts[0].trove_manager,
        &contracts.active_pool,
        1_100 * PRECISION,
        1_000 * PRECISION,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .unwrap();

    trove_manager_abi::liquidate(
        &contracts.aswith_contracts[0].trove_manager,
        &contracts.community_issuance,
        &contracts.stability_pool,
        &contracts.aswith_contracts[0].oracle,
        &contracts.sorted_troves,
        &contracts.active_pool,
        &contracts.default_pool,
        &contracts.coll_surplus_pool,
        &contracts.usdf,
        Identity::Address(wallet1.address().into()),
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .expect_err("Improper liquidation of trove not below MCR");
}
