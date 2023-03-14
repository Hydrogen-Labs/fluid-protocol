use fuels::types::Identity;
use test_utils::{
    interfaces::{
        borrow_operations::{borrow_operations_abi, BorrowOperations},
        default_pool::default_pool_abi,
        oracle::oracle_abi,
        stability_pool::{stability_pool_abi, StabilityPool},
        token::token_abi,
        trove_manager::{trove_manager_abi, Status},
    },
    setup::common::setup_protocol,
};

#[tokio::test]
async fn proper_partial_liquidation_enough_usdf_in_sp() {
    let (contracts, _admin, mut wallets) = setup_protocol(10, 5).await;

    oracle_abi::set_price(&contracts.oracle, 10_000_000).await;

    let wallet1 = wallets.pop().unwrap();
    let wallet2 = wallets.pop().unwrap();

    let balance = 25_000_000_000;
    token_abi::mint_to_id(
        &contracts.fuel,
        balance,
        Identity::Address(wallet1.address().into()),
    )
    .await;

    token_abi::mint_to_id(
        &contracts.fuel,
        balance,
        Identity::Address(wallet2.address().into()),
    )
    .await;

    let borrow_operations_wallet1 = BorrowOperations::new(
        contracts.borrow_operations.contract_id().clone(),
        wallet1.clone(),
    );

    let borrow_operations_wallet2 = BorrowOperations::new(
        contracts.borrow_operations.contract_id().clone(),
        wallet2.clone(),
    );

    borrow_operations_abi::open_trove(
        &borrow_operations_wallet1,
        &contracts.oracle,
        &contracts.fuel,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.trove_manager,
        &contracts.active_pool,
        12_000_000_000,
        10_100_000_000,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .unwrap();

    // Open 2nd trove to deposit into stability pool
    borrow_operations_abi::open_trove(
        &borrow_operations_wallet2,
        &contracts.oracle,
        &contracts.fuel,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.trove_manager,
        &contracts.active_pool,
        20_000_000_000,
        15_000_000_000,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .unwrap();

    let stability_pool_wallet2 = StabilityPool::new(
        contracts.stability_pool.contract_id().clone(),
        wallet2.clone(),
    );

    stability_pool_abi::provide_to_stability_pool(
        &stability_pool_wallet2,
        &contracts.usdf,
        15_000_000_000,
    )
    .await
    .unwrap();

    oracle_abi::set_price(&contracts.oracle, 1_000_000).await;
    // Wallet 1 has collateral ratio of 110% and wallet 2 has 200% so we can liquidate it

    trove_manager_abi::liquidate(
        &contracts.trove_manager,
        &contracts.stability_pool,
        &contracts.oracle,
        &contracts.sorted_troves,
        &contracts.active_pool,
        &contracts.default_pool,
        Identity::Address(wallet1.address().into()),
    )
    .await
    .unwrap();

    let status = trove_manager_abi::get_trove_status(
        &contracts.trove_manager,
        Identity::Address(wallet1.address().into()),
    )
    .await
    .unwrap()
    .value;

    assert_eq!(status, Status::Active);

    let coll = trove_manager_abi::get_trove_coll(
        &contracts.trove_manager,
        Identity::Address(wallet1.address().into()),
    )
    .await
    .value;

    let debt = trove_manager_abi::get_trove_debt(
        &contracts.trove_manager,
        Identity::Address(wallet1.address().into()),
    )
    .await
    .value;

    let collateral_ratio = coll * 1_000_000 / debt;

    let default_pool_asset = default_pool_abi::get_asset(&contracts.default_pool)
        .await
        .value;

    let default_pool_debt = default_pool_abi::get_usdf_debt(&contracts.default_pool)
        .await
        .value;

    assert_eq!(default_pool_asset, 0);
    assert_eq!(default_pool_debt, 0);
    assert_eq!(collateral_ratio, 1_300_000);

    let pending_asset_rewards = trove_manager_abi::get_pending_asset_reward(
        &contracts.trove_manager,
        Identity::Address(wallet1.address().into()),
    )
    .await
    .value;

    let pending_usdf_rewards = trove_manager_abi::get_pending_usdf_reward(
        &contracts.trove_manager,
        Identity::Address(wallet1.address().into()),
    )
    .await
    .value;

    assert_eq!(pending_asset_rewards, 0);
    assert_eq!(pending_usdf_rewards, 0);
}