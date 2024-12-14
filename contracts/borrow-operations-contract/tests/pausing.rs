use fuels::{prelude::*, types::Identity};

use test_utils::{
    data_structures::{ContractInstance, PRECISION},
    interfaces::{
        borrow_operations::{borrow_operations_abi, BorrowOperations},
        oracle::oracle_abi,
        pyth_oracle::{pyth_oracle_abi, pyth_price_feed, PYTH_TIMESTAMP},
        token::token_abi,
    },
    setup::common::setup_protocol,
};

#[tokio::test]
async fn test_permissions() {
    let (contracts, admin, mut wallets) = setup_protocol(5, false, false).await;

    // Test set_pauser
    let new_pauser = wallets.pop().unwrap();
    let result = borrow_operations_abi::set_pauser(
        &contracts.borrow_operations,
        Identity::Address(new_pauser.address().into()),
    )
    .await;
    let borrow_operations_new_pauser = ContractInstance::new(
        BorrowOperations::new(
            contracts.borrow_operations.contract.contract_id().clone(),
            new_pauser.clone(),
        ),
        contracts.borrow_operations.implementation_id.clone(),
    );
    assert!(result.is_ok(), "Admin should be able to set a new pauser");

    // Verify unauthorized set_pauser
    let unauthorized_wallet = wallets.pop().unwrap();
    let unauthorized_borrow_operations = ContractInstance::new(
        BorrowOperations::new(
            contracts.borrow_operations.contract.contract_id().clone(),
            unauthorized_wallet.clone(),
        ),
        contracts.borrow_operations.implementation_id.clone(),
    );
    let result = borrow_operations_abi::set_pauser(
        &unauthorized_borrow_operations,
        Identity::Address(unauthorized_wallet.address().into()),
    )
    .await;
    assert!(
        result.is_err(),
        "Unauthorized wallet should not be able to set pauser"
    );

    // Test transfer_owner
    let new_owner = wallets.pop().unwrap();
    let result = borrow_operations_abi::transfer_owner(
        &contracts.borrow_operations,
        Identity::Address(new_owner.address().into()),
    )
    .await;
    assert!(result.is_ok(), "Admin should be able to transfer ownership");

    // Verify old owner can't perform admin actions
    let result = borrow_operations_abi::set_pauser(
        &contracts.borrow_operations,
        Identity::Address(admin.address().into()),
    )
    .await;
    assert!(
        result.is_err(),
        "Old owner should not be able to set pauser after transfer"
    );

    // Test renounce_owner
    let new_borrow_operations = ContractInstance::new(
        BorrowOperations::new(
            contracts.borrow_operations.contract.contract_id().clone(),
            new_owner.clone(),
        ),
        contracts.borrow_operations.implementation_id.clone(),
    );
    let result = borrow_operations_abi::renounce_owner(&new_borrow_operations).await;
    assert!(
        result.is_ok(),
        "New owner should be able to renounce ownership"
    );

    // Verify no owner can perform admin actions
    let result = borrow_operations_abi::set_pauser(
        &new_borrow_operations,
        Identity::Address(new_owner.address().into()),
    )
    .await;
    assert!(
        result.is_err(),
        "No owner should be able to set pauser after renouncement"
    );

    let pauser = borrow_operations_abi::get_pauser(&contracts.borrow_operations)
        .await
        .unwrap();
    assert_eq!(
        pauser.value,
        Identity::Address(new_pauser.address().into()),
        "Pauser should be the new pauser"
    );

    // Test setting pause status to true
    let _ = borrow_operations_abi::set_pause_status(&borrow_operations_new_pauser, true)
        .await
        .unwrap();

    let status = borrow_operations_abi::get_is_paused(&contracts.borrow_operations)
        .await
        .unwrap();
    assert!(status.value, "Failed to set pause status to true");

    // Test setting pause status to false
    let _ = borrow_operations_abi::set_pause_status(&borrow_operations_new_pauser, false)
        .await
        .unwrap();
    let status = borrow_operations_abi::get_is_paused(&contracts.borrow_operations)
        .await
        .unwrap();
    assert!(!status.value, "Failed to set pause status to false");

    let unauthorized_wallet = wallets.pop().unwrap();
    let unauthorized_borrow_operations = ContractInstance::new(
        BorrowOperations::new(
            contracts.borrow_operations.contract.contract_id().clone(),
            unauthorized_wallet.clone(),
        ),
        contracts.borrow_operations.implementation_id.clone(),
    );

    // Try to set pause status with unauthorized wallet
    let res = borrow_operations_abi::set_pause_status(&unauthorized_borrow_operations, true).await;

    assert!(
        res.is_err(),
        "Unauthorized wallet should not be able to set pause status"
    );
}

#[tokio::test]
async fn test_paused_operations() {
    let (contracts, admin, _) = setup_protocol(2, true, false).await;

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    token_abi::mint_to_id(
        &contracts.asset_contracts[1].asset,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    let deposit_amount = 1_200 * PRECISION;
    let borrow_amount = 600 * PRECISION;

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(1),
    )
    .await;

    // Open a trove while unpaused
    borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        deposit_amount,
        borrow_amount,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    // Set pause status to true
    borrow_operations_abi::set_pause_status(&contracts.borrow_operations, true)
        .await
        .unwrap();

    // Try to open another trove while paused
    let res = borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.asset_contracts[1].oracle,
        &contracts.asset_contracts[1].mock_pyth_oracle,
        &contracts.asset_contracts[1].mock_redstone_oracle,
        &contracts.asset_contracts[1].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[1].trove_manager,
        &contracts.active_pool,
        deposit_amount,
        borrow_amount,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await;

    assert!(
        res.is_err(),
        "Should not be able to open trove while paused"
    );

    // Try to withdraw USDF (increase debt) while paused
    let withdraw_amount = 100 * PRECISION;
    let res = borrow_operations_abi::withdraw_usdf(
        &contracts.borrow_operations,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        withdraw_amount,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await;

    assert!(
        res.is_err(),
        "Should not be able to withdraw USDF while paused"
    );

    // Try to repay USDF (reduce debt) while paused
    let repay_amount = 100 * PRECISION;
    let res = borrow_operations_abi::repay_usdf(
        &contracts.borrow_operations,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        &contracts.default_pool,
        repay_amount,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await;

    assert!(res.is_ok(), "Should be able to repay USDF while paused");

    // Try to add collateral while paused
    let res = borrow_operations_abi::add_coll(
        &contracts.borrow_operations,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        1_000 * PRECISION,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await;

    assert!(res.is_ok(), "Should be able to add collateral while paused");

    // Try to withdraw collateral while paused
    let res = borrow_operations_abi::withdraw_coll(
        &contracts.borrow_operations,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        1_000 * PRECISION,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await;

    assert!(
        res.is_ok(),
        "Should be able to withdraw collateral while paused"
    );

    // Set pause status to false
    borrow_operations_abi::set_pause_status(&contracts.borrow_operations, false)
        .await
        .unwrap();

    // Try to withdraw USDF while unpaused
    let res = borrow_operations_abi::withdraw_usdf(
        &contracts.borrow_operations,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        withdraw_amount,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await;

    assert!(
        res.is_ok(),
        "Should be able to withdraw USDF while unpaused"
    );
}

#[tokio::test]
async fn test_asset_specific_pausing() {
    let (contracts, admin, mut wallets) = setup_protocol(2, true, false).await;
    let asset_0 = contracts.asset_contracts[0].asset_id;
    let asset_1 = contracts.asset_contracts[1].asset_id;

    // Test setting pause status for specific asset
    let result =
        borrow_operations_abi::set_asset_pause_status(&contracts.borrow_operations, asset_0, true)
            .await;
    assert!(
        result.is_ok(),
        "Admin should be able to pause specific asset"
    );

    // Verify pause status
    let status =
        borrow_operations_abi::get_asset_pause_status(&contracts.borrow_operations, asset_0)
            .await
            .unwrap();
    assert!(status.value, "Asset 0 should be paused");

    let status =
        borrow_operations_abi::get_asset_pause_status(&contracts.borrow_operations, asset_1)
            .await
            .unwrap();
    assert!(!status.value, "Asset 1 should not be paused");

    // Setup for trove operations
    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    token_abi::mint_to_id(
        &contracts.asset_contracts[1].asset,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    let deposit_amount = 1_200 * PRECISION;
    let borrow_amount = 600 * PRECISION;

    // Setup price feeds
    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(1),
    )
    .await;

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[1].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[1].mock_pyth_oracle,
        pyth_price_feed(1),
    )
    .await;

    // Try to open trove with paused asset (should fail)
    let res = borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        deposit_amount,
        borrow_amount,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await;

    assert!(
        res.is_err(),
        "Should not be able to open trove with paused asset"
    );

    // Should be able to open trove with unpaused asset
    let res = borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.asset_contracts[1].oracle,
        &contracts.asset_contracts[1].mock_pyth_oracle,
        &contracts.asset_contracts[1].mock_redstone_oracle,
        &contracts.asset_contracts[1].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[1].trove_manager,
        &contracts.active_pool,
        deposit_amount,
        borrow_amount,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await;

    assert!(
        res.is_ok(),
        "Should be able to open trove with unpaused asset"
    );

    // Test unauthorized pause attempt
    let unauthorized_wallet = wallets.pop().unwrap();
    let unauthorized_borrow_operations = ContractInstance::new(
        BorrowOperations::new(
            contracts.borrow_operations.contract.contract_id().clone(),
            unauthorized_wallet.clone(),
        ),
        contracts.borrow_operations.implementation_id.clone(),
    );

    let res = borrow_operations_abi::set_asset_pause_status(
        &unauthorized_borrow_operations,
        asset_1,
        true,
    )
    .await;

    assert!(
        res.is_err(),
        "Unauthorized wallet should not be able to pause asset"
    );
}

#[tokio::test]
async fn test_paused_asset_operations() {
    let (contracts, admin, _) = setup_protocol(1, true, false).await;
    let asset_0 = contracts.asset_contracts[0].asset_id;

    // Setup initial trove
    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    let deposit_amount = 1_200 * PRECISION;
    let borrow_amount = 600 * PRECISION;

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(1),
    )
    .await;

    // Open trove before pausing
    borrow_operations_abi::open_trove(
        &contracts.borrow_operations,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        deposit_amount,
        borrow_amount,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    // Pause the asset
    borrow_operations_abi::set_asset_pause_status(&contracts.borrow_operations, asset_0, true)
        .await
        .unwrap();

    // Try to withdraw USDF (increase debt) while asset is paused
    let withdraw_amount = 100 * PRECISION;
    let res = borrow_operations_abi::withdraw_usdf(
        &contracts.borrow_operations,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        withdraw_amount,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await;

    assert!(
        res.is_err(),
        "Should not be able to withdraw USDF while asset is paused"
    );

    // Should still be able to repay USDF while asset is paused
    let repay_amount = 100 * PRECISION;
    let res = borrow_operations_abi::repay_usdf(
        &contracts.borrow_operations,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        &contracts.default_pool,
        repay_amount,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await;

    assert!(
        res.is_ok(),
        "Should be able to repay USDF while asset is paused"
    );
}
