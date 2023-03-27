use fuels::{prelude::*, types::Identity};

use test_utils::{
    interfaces::borrow_operations::borrow_operations_abi,
    interfaces::sorted_troves::sorted_troves_abi,
    interfaces::{active_pool::active_pool_abi, token::token_abi},
    interfaces::{trove_manager::trove_manager_abi, usdf_token::usdf_token_abi},
    setup::common::{deploy_token, deploy_usdf_token, setup_protocol},
    utils::{calculate_icr, with_min_borrow_fee},
};

#[tokio::test]
async fn fails_open_two_troves() {
    let (contracts, admin, _) = setup_protocol(100, 2).await;

    token_abi::mint_to_id(
        &contracts.fuel,
        5_000_000_000,
        Identity::Address(admin.address().into()),
    )
    .await;

    let provider = admin.get_provider().unwrap();

    borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.oracle,
        &contracts.fuel,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.trove_manager,
        &contracts.active_pool,
        1_200_000_000,
        600_000_000,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .unwrap();

    let res = borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.oracle,
        &contracts.fuel,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.trove_manager,
        &contracts.active_pool,
        1_200_000_000,
        600_000_000,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .is_err();

    assert!(res);

    let usdf_balance = provider
        .get_asset_balance(
            admin.address().into(),
            AssetId::from(*contracts.usdf.contract_id().hash()),
        )
        .await
        .unwrap();

    let first = sorted_troves_abi::get_first(
        &contracts.sorted_troves,
        contracts.fuel.contract_id().into(),
    )
    .await
    .value;
    let last = sorted_troves_abi::get_last(
        &contracts.sorted_troves,
        contracts.fuel.contract_id().into(),
    )
    .await
    .value;
    let size = sorted_troves_abi::get_size(&contracts.sorted_troves)
        .await
        .value;
    let icr = trove_manager_abi::get_nominal_icr(
        &contracts.trove_manager,
        Identity::Address(admin.address().into()),
    )
    .await
    .value;

    assert_eq!(size, 1);
    assert_eq!(first, Identity::Address(admin.address().into()));
    assert_eq!(last, Identity::Address(admin.address().into()));
    assert_eq!(usdf_balance, 600_000_000);

    let expected_debt = with_min_borrow_fee(600_000_000);
    let expected_icr = calculate_icr(1_200_000_000, expected_debt);

    assert_eq!(icr, expected_icr, "ICR is wrong");

    let trove_col = trove_manager_abi::get_trove_coll(
        &contracts.trove_manager,
        Identity::Address(admin.address().into()),
    )
    .await
    .value;

    let trove_debt = trove_manager_abi::get_trove_debt(
        &contracts.trove_manager,
        Identity::Address(admin.address().into()),
    )
    .await
    .value;

    assert_eq!(trove_col, 1_200_000_000, "Trove Collateral is wrong");
    assert_eq!(trove_debt, expected_debt, "Trove Debt is wrong");

    let active_pool_debt = active_pool_abi::get_usdf_debt(&contracts.active_pool)
        .await
        .value;
    assert_eq!(active_pool_debt, expected_debt, "Active Pool Debt is wrong");

    let active_pool_col =
        active_pool_abi::get_asset(&contracts.active_pool, contracts.fuel.contract_id().into())
            .await
            .value;
    assert_eq!(
        active_pool_col, 1_200_000_000,
        "Active Pool Collateral is wrong"
    );
}

#[tokio::test]
async fn fails_open_trove_under_minimum_collateral_ratio() {
    // MCR = 1_200_000
    let (contracts, admin, _) = setup_protocol(100, 2).await;

    token_abi::mint_to_id(
        &contracts.fuel,
        5_000_000_000,
        Identity::Address(admin.address().into()),
    )
    .await;

    let res = borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.oracle,
        &contracts.fuel,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.trove_manager,
        &contracts.active_pool,
        1_200_000_000,
        1_000_000_000,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .is_err();

    assert!(
        res,
        "Borrow operation: Should not be able to open trove with MCR < 1.2"
    );
}

#[tokio::test]
async fn fails_open_trove_under_min_usdf_required() {
    // MCR = 1_200_000
    let (contracts, admin, _) = setup_protocol(100, 2).await;

    token_abi::mint_to_id(
        &contracts.fuel,
        5_000_000_000,
        Identity::Address(admin.address().into()),
    )
    .await;

    let res = borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.oracle,
        &contracts.fuel,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.trove_manager,
        &contracts.active_pool,
        1_200_000_000,
        100_000_000,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .is_err();

    assert!(
        res,
        "Borrow operation: Should not be able to open trove with MCR < 1.2"
    );
}

#[tokio::test]
async fn fails_reduce_debt_under_min_usdf_required() {
    // MCR = 1_200_000
    let (contracts, admin, _) = setup_protocol(100, 2).await;

    token_abi::mint_to_id(
        &contracts.fuel,
        5_000_000_000,
        Identity::Address(admin.address().into()),
    )
    .await;

    borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.oracle,
        &contracts.fuel,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.trove_manager,
        &contracts.active_pool,
        1_200_000_000,
        600_000_000,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .unwrap();

    let res = borrow_operations_abi::repay_usdf(
        &contracts.borrow_operations,
        &contracts.oracle,
        &contracts.fuel,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.trove_manager,
        &contracts.active_pool,
        300_000_000,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .is_err();

    assert!(
        res,
        "Borrow operation: Should not be able to reduce debt to less than 500 USDF"
    );
}

#[tokio::test]
async fn fails_decrease_collateral_under_mcr() {
    // MCR = 1_200_000
    let (contracts, admin, _) = setup_protocol(100, 2).await;

    token_abi::mint_to_id(
        &contracts.fuel,
        5_000_000_000,
        Identity::Address(admin.address().into()),
    )
    .await;

    borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.oracle,
        &contracts.fuel,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.trove_manager,
        &contracts.active_pool,
        1_200_000_000,
        600_000_000,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .unwrap();

    let res = borrow_operations_abi::withdraw_coll(
        &contracts.borrow_operations,
        &contracts.oracle,
        &contracts.fuel,
        &contracts.sorted_troves,
        &contracts.trove_manager,
        &contracts.active_pool,
        1_000_000_000,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .is_err();

    assert!(
        res,
        "Borrow operation: Should not be able to reduce collateral to less than 1.2 MCR"
    );
}

#[tokio::test]
async fn fails_incorrect_token_as_collateral_or_repayment() {
    // MCR = 1_200_000
    let (contracts, admin, _) = setup_protocol(100, 2).await;

    let mock_fake_token = deploy_token(&admin).await;

    token_abi::initialize(
        &mock_fake_token,
        0,
        &Identity::Address(admin.address().into()),
        "Fake Fuel".to_string(),
        "FFUEL".to_string(),
    )
    .await;

    token_abi::mint_to_id(
        &mock_fake_token,
        5_000_000_000,
        Identity::Address(admin.address().into()),
    )
    .await;

    token_abi::mint_to_id(
        &contracts.fuel,
        5_000_000_000,
        Identity::Address(admin.address().into()),
    )
    .await;

    let res = borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.oracle,
        &mock_fake_token,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.trove_manager,
        &contracts.active_pool,
        1_200_000_000,
        600_000_000,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .is_err();

    assert!(
        res,
        "Borrow operation: Should not be able to open trove with incorrect token as collateral"
    );

    // Set up real trove and try to add collateral
    borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.oracle,
        &contracts.fuel,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.trove_manager,
        &contracts.active_pool,
        1_200_000_000,
        600_000_000,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .unwrap();

    let res = borrow_operations_abi::add_coll(
        &contracts.borrow_operations,
        &contracts.oracle,
        &mock_fake_token,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.trove_manager,
        &contracts.active_pool,
        1_000_000,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .is_err();

    assert!(
        res,
        "Borrow operation: Should not be able to add collateral with incorrect token as collateral"
    );

    let fake_usdf_token = deploy_usdf_token(&admin).await;

    usdf_token_abi::initialize(
        &fake_usdf_token,
        "Fake USDF".to_string(),
        "FUSDF".to_string(),
        Identity::Address(admin.address().into()),
        Identity::Address(admin.address().into()),
        Identity::Address(admin.address().into()),
    )
    .await;

    usdf_token_abi::mint(
        &fake_usdf_token,
        5_000_000_000,
        Identity::Address(admin.address().into()),
    )
    .await
    .unwrap();

    let res = borrow_operations_abi::repay_usdf(
        &contracts.borrow_operations,
        &contracts.oracle,
        &contracts.fuel,
        &fake_usdf_token,
        &contracts.sorted_troves,
        &contracts.trove_manager,
        &contracts.active_pool,
        1_000_000,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .is_err();

    assert!(
        res,
        "Borrow operation: Should not be able to repay with incorrect token as repayment"
    );
}
