use fuels::{prelude::*, types::Identity};

use test_utils::{
    data_structures::PRECISION,
    interfaces::borrow_operations::borrow_operations_abi,
    interfaces::sorted_troves::sorted_troves_abi,
    interfaces::{active_pool::active_pool_abi, token::token_abi},
    interfaces::{trove_manager::trove_manager_abi, usdf_token::usdf_token_abi},
    setup::common::{deploy_token, deploy_usdf_token, setup_protocol},
    utils::{calculate_icr, with_min_borrow_fee},
};

#[tokio::test]
async fn fails_open_two_troves_of_same_coll_type() {
    let (contracts, admin, _) = setup_protocol(100, 2, false).await;

    token_abi::mint_to_id(
        &contracts.aswith_contracts[0].asset,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    let provider = admin.provider().unwrap();

    let col_amount = 1_200 * PRECISION;
    let debt_amount = 600 * PRECISION;

    borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.aswith_contracts[0].oracle,
        &contracts.aswith_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.aswith_contracts[0].trove_manager,
        &contracts.active_pool,
        col_amount,
        debt_amount,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .unwrap();

    let res = borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.aswith_contracts[0].oracle,
        &contracts.aswith_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.aswith_contracts[0].trove_manager,
        &contracts.active_pool,
        col_amount,
        debt_amount,
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

    let asset: ContractId = contracts.aswith_contracts[0].asset.contract_id().into();

    let first = sorted_troves_abi::get_first(&contracts.sorted_troves, asset)
        .await
        .value;
    let last = sorted_troves_abi::get_last(&contracts.sorted_troves, asset)
        .await
        .value;
    let size = sorted_troves_abi::get_size(&contracts.sorted_troves, asset)
        .await
        .value;
    let icr = trove_manager_abi::get_nominal_icr(
        &contracts.aswith_contracts[0].trove_manager,
        Identity::Address(admin.address().into()),
    )
    .await
    .value;

    assert_eq!(size, 1);
    assert_eq!(first, Identity::Address(admin.address().into()));
    assert_eq!(last, Identity::Address(admin.address().into()));
    assert_eq!(usdf_balance, debt_amount);

    let expected_debt = with_min_borrow_fee(debt_amount);
    let expected_icr = calculate_icr(col_amount, expected_debt);

    assert_eq!(icr, expected_icr, "ICR is wrong");

    let trove_col = trove_manager_abi::get_trove_coll(
        &contracts.aswith_contracts[0].trove_manager,
        Identity::Address(admin.address().into()),
    )
    .await
    .value;

    let trove_debt = trove_manager_abi::get_trove_debt(
        &contracts.aswith_contracts[0].trove_manager,
        Identity::Address(admin.address().into()),
    )
    .await
    .value;

    assert_eq!(trove_col, col_amount, "Trove Collateral is wrong");
    assert_eq!(trove_debt, expected_debt, "Trove Debt is wrong");

    let active_pool_debt = active_pool_abi::get_usdf_debt(
        &contracts.active_pool,
        contracts.aswith_contracts[0].asset.contract_id().into(),
    )
    .await
    .value;
    assert_eq!(active_pool_debt, expected_debt, "Active Pool Debt is wrong");

    let active_pool_col = active_pool_abi::get_asset(
        &contracts.active_pool,
        contracts.aswith_contracts[0].asset.contract_id().into(),
    )
    .await
    .value;
    assert_eq!(
        active_pool_col, col_amount,
        "Active Pool Collateral is wrong"
    );
}

#[tokio::test]
async fn fails_open_trove_under_minimum_collateral_ratio() {
    let (contracts, admin, _) = setup_protocol(100, 2, false).await;

    token_abi::mint_to_id(
        &contracts.aswith_contracts[0].asset,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    // 120% Collateral Ratio < 130% Minimum Collateral Ratio
    let coll_amount = 1200 * PRECISION;
    let debt_amount = 1000 * PRECISION;

    let res = borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.aswith_contracts[0].oracle,
        &contracts.aswith_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.aswith_contracts[0].trove_manager,
        &contracts.active_pool,
        coll_amount,
        debt_amount,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .is_err();

    assert!(
        res,
        "Borrow operation: Should not be able to open trove with MCR < 130%"
    );
}

#[tokio::test]
async fn fails_open_trove_under_min_usdf_required() {
    let (contracts, admin, _) = setup_protocol(100, 2, false).await;

    token_abi::mint_to_id(
        &contracts.aswith_contracts[0].asset,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    let coll_amount = 1_200 * PRECISION;
    let debt_amount = 400 * PRECISION;
    // 100 USDF < 500 USDF

    let res = borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.aswith_contracts[0].oracle,
        &contracts.aswith_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.aswith_contracts[0].trove_manager,
        &contracts.active_pool,
        coll_amount,
        debt_amount,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .is_err();

    assert!(
        res,
        "Borrow operation: Should not be able to open trove with debt < 500 USDF"
    );
}

#[tokio::test]
async fn fails_reduce_debt_under_min_usdf_required() {
    let (contracts, admin, _) = setup_protocol(100, 2, false).await;

    token_abi::mint_to_id(
        &contracts.aswith_contracts[0].asset,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    let coll_amount = 1_200 * PRECISION;
    let debt_amount = 600 * PRECISION;

    borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.aswith_contracts[0].oracle,
        &contracts.aswith_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.aswith_contracts[0].trove_manager,
        &contracts.active_pool,
        coll_amount,
        debt_amount,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .unwrap();

    // 600 USDF - 300 USDF < 500 USDF

    let res = borrow_operations_abi::repay_usdf(
        &contracts.borrow_operations,
        &contracts.aswith_contracts[0].oracle,
        &contracts.aswith_contracts[0].asset,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.aswith_contracts[0].trove_manager,
        &contracts.active_pool,
        300 * PRECISION,
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
    let (contracts, admin, _) = setup_protocol(100, 2, false).await;

    token_abi::mint_to_id(
        &contracts.aswith_contracts[0].asset,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    let coll_amount = 1_200 * PRECISION;
    let debt_amount = 600 * PRECISION;

    borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.aswith_contracts[0].oracle,
        &contracts.aswith_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.aswith_contracts[0].trove_manager,
        &contracts.active_pool,
        coll_amount,
        debt_amount,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .unwrap();

    let res = borrow_operations_abi::withdraw_coll(
        &contracts.borrow_operations,
        &contracts.aswith_contracts[0].oracle,
        &contracts.aswith_contracts[0].asset,
        &contracts.sorted_troves,
        &contracts.aswith_contracts[0].trove_manager,
        &contracts.active_pool,
        600 * PRECISION,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .is_err();

    assert!(
        res,
        "Borrow operation: Should not be able to reduce collateral to less than 130% MCR"
    );
}

#[tokio::test]
async fn fails_incorrect_token_as_collateral_or_repayment() {
    let (contracts, admin, _) = setup_protocol(100, 2, false).await;

    let mock_fake_token = deploy_token(&admin).await;

    token_abi::initialize(
        &mock_fake_token,
        0,
        &Identity::Address(admin.address().into()),
        "Fake Coll".to_string(),
        "FCOL".to_string(),
    )
    .await;

    token_abi::mint_to_id(
        &mock_fake_token,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    token_abi::mint_to_id(
        &contracts.aswith_contracts[0].asset,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    let res = borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.aswith_contracts[0].oracle,
        &mock_fake_token,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.aswith_contracts[0].trove_manager,
        &contracts.active_pool,
        1_200 * PRECISION,
        600 * PRECISION,
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
        &contracts.aswith_contracts[0].oracle,
        &contracts.aswith_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.aswith_contracts[0].trove_manager,
        &contracts.active_pool,
        1_200 * PRECISION,
        600 * PRECISION,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .unwrap();

    let res = borrow_operations_abi::add_coll(
        &contracts.borrow_operations,
        &contracts.aswith_contracts[0].oracle,
        &mock_fake_token,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.aswith_contracts[0].trove_manager,
        &contracts.active_pool,
        1 * PRECISION,
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
        fake_usdf_token.contract_id().into(),
        Identity::Address(admin.address().into()),
        Identity::Address(admin.address().into()),
    )
    .await;

    usdf_token_abi::mint(
        &fake_usdf_token,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await
    .unwrap();

    let res = borrow_operations_abi::repay_usdf(
        &contracts.borrow_operations,
        &contracts.aswith_contracts[0].oracle,
        &contracts.aswith_contracts[0].asset,
        &fake_usdf_token,
        &contracts.sorted_troves,
        &contracts.aswith_contracts[0].trove_manager,
        &contracts.active_pool,
        1 * PRECISION,
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
