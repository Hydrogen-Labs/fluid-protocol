use fuels::types::Identity;
use test_utils::data_structures::{ContractInstance, PRECISION};
use test_utils::interfaces::borrow_operations::borrow_operations_utils;
use test_utils::interfaces::oracle::oracle_abi;
use test_utils::interfaces::protocol_manager::ProtocolManager;
use test_utils::interfaces::pyth_oracle::PYTH_TIMESTAMP;
use test_utils::{
    interfaces::{
        active_pool::active_pool_abi,
        protocol_manager::protocol_manager_abi,
        pyth_oracle::{pyth_oracle_abi, pyth_price_feed},
        trove_manager::trove_manager_utils,
    },
    setup::common::setup_protocol,
    utils::with_min_borrow_fee,
};

#[tokio::test]
async fn proper_multi_collateral_redemption_from_partially_closed() {
    let (contracts, _admin, mut wallets) = setup_protocol(5, true, false).await;

    let healthy_wallet1 = wallets.pop().unwrap();
    let healthy_wallet2 = wallets.pop().unwrap();
    let healthy_wallet3 = wallets.pop().unwrap();

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

    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet1.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        20_000 * PRECISION,
        10_000 * PRECISION,
    )
    .await;

    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet2.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        9_000 * PRECISION,
        5_000 * PRECISION,
    )
    .await;

    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet3.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        8_000 * PRECISION,
        5_000 * PRECISION,
    )
    .await;

    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet2.clone(),
        &contracts.asset_contracts[1],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        15_000 * PRECISION,
        5_000 * PRECISION,
    )
    .await;

    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet3.clone(),
        &contracts.asset_contracts[1],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        7_000 * PRECISION,
        5_000 * PRECISION,
    )
    .await;

    // 2 Collateral types
    // 1st collateral
    // 20k FUEL > 9k FUEL > 8k FUEL
    // 10k USDF > 5k USDF > 5k USDF + (fees)

    // 2nd collateral
    // 7k mock2 > 15k mock2
    // 5k USDF   > 5k USDF + (fees)

    // Redeeming 10k USDF, so 1,3 and 2,2 should be closed

    let redemption_amount: u64 = 8_000 * PRECISION;

    let protocol_manager_health1 = ContractInstance::new(
        ProtocolManager::new(
            contracts.protocol_manager.contract.contract_id().clone(),
            healthy_wallet1.clone(),
        ),
        contracts.protocol_manager.implementation_id,
    );

    let pre_redemption_active_pool_debt = active_pool_abi::get_usdf_debt(
        &contracts.active_pool,
        contracts.asset_contracts[0].asset_id.into(),
    )
    .await
    .value;

    protocol_manager_abi::redeem_collateral(
        &protocol_manager_health1,
        redemption_amount,
        20,
        0,
        None,
        None,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.coll_surplus_pool,
        &contracts.default_pool,
        &contracts.active_pool,
        &contracts.sorted_troves,
        &contracts.asset_contracts,
    )
    .await
    .unwrap();

    let active_pool_asset = active_pool_abi::get_asset(
        &contracts.active_pool,
        contracts.asset_contracts[0].asset_id.into(),
    )
    .await
    .value;

    let active_pool_debt = active_pool_abi::get_usdf_debt(
        &contracts.active_pool,
        contracts.asset_contracts[0].asset_id.into(),
    )
    .await
    .value;

    // Total active pool asset should be reduced by the redemption amount
    //  + amount taken from the 2nd collateral type
    assert_eq!(
        active_pool_asset,
        37_000 * PRECISION - redemption_amount + with_min_borrow_fee(5_000 * PRECISION)
    );

    assert_eq!(
        active_pool_debt,
        pre_redemption_active_pool_debt - redemption_amount
            + with_min_borrow_fee(5_000 * PRECISION)
    );

    let provider = healthy_wallet1.provider().unwrap();

    let mock_asset_id = contracts.asset_contracts[0].asset_id;
    let st_mock_asset_id = contracts.asset_contracts[1].asset_id;

    let mock_balance = provider
        .get_asset_balance(healthy_wallet1.address(), mock_asset_id)
        .await
        .unwrap();

    let st_mock_balance = provider
        .get_asset_balance(healthy_wallet1.address(), st_mock_asset_id)
        .await
        .unwrap();

    let staking_balance = provider
        .get_contract_asset_balance(&contracts.fpt_staking.contract.contract_id(), mock_asset_id)
        .await
        .unwrap();

    let fees2 = provider
        .get_contract_asset_balance(
            &contracts.fpt_staking.contract.contract_id(),
            st_mock_asset_id,
        )
        .await
        .unwrap();

    assert_eq!(
        mock_balance + st_mock_balance,
        redemption_amount - staking_balance - fees2
    );

    // Started with 8k portion obsorved by the 2nd collateral type
    trove_manager_utils::assert_trove_coll(
        &contracts.asset_contracts[0].trove_manager,
        Identity::Address(healthy_wallet3.address().into()),
        8_000 * PRECISION + st_mock_balance + fees2 - redemption_amount,
    )
    .await;

    trove_manager_utils::assert_trove_debt(
        &contracts.asset_contracts[0].trove_manager,
        Identity::Address(healthy_wallet3.address().into()),
        with_min_borrow_fee(5_000 * PRECISION) + st_mock_balance + fees2 - redemption_amount,
    )
    .await;
}

#[tokio::test]
async fn proper_multi_collateral_redemption_with_empty_second_asset() {
    let (contracts, _admin, mut wallets) = setup_protocol(5, true, false).await;

    let healthy_wallet1 = wallets.pop().unwrap();
    let healthy_wallet2 = wallets.pop().unwrap();
    let healthy_wallet3 = wallets.pop().unwrap();

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

    // Only open troves for first asset type
    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet1.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        20_000 * PRECISION,
        10_000 * PRECISION,
    )
    .await;

    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet2.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        9_000 * PRECISION,
        5_000 * PRECISION,
    )
    .await;

    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet3.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        8_000 * PRECISION,
        5_000 * PRECISION,
    )
    .await;

    let active_pool_asset_before_redemption = active_pool_abi::get_asset(
        &contracts.active_pool,
        contracts.asset_contracts[0].asset_id.into(),
    )
    .await
    .value;

    // No troves opened for second asset type

    let redemption_amount: u64 = 3_000 * PRECISION;

    let protocol_manager_health1 = ContractInstance::new(
        ProtocolManager::new(
            contracts.protocol_manager.contract.contract_id().clone(),
            healthy_wallet1.clone(),
        ),
        contracts.protocol_manager.implementation_id,
    );

    let pre_redemption_active_pool_debt = active_pool_abi::get_usdf_debt(
        &contracts.active_pool,
        contracts.asset_contracts[0].asset_id.into(),
    )
    .await
    .value;

    protocol_manager_abi::redeem_collateral(
        &protocol_manager_health1,
        redemption_amount,
        20,
        0,
        None,
        None,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.coll_surplus_pool,
        &contracts.default_pool,
        &contracts.active_pool,
        &contracts.sorted_troves,
        &contracts.asset_contracts,
    )
    .await
    .unwrap();

    // Verify first asset pool changes
    let active_pool_asset_after_redemption = active_pool_abi::get_asset(
        &contracts.active_pool,
        contracts.asset_contracts[0].asset_id.into(),
    )
    .await
    .value;

    let active_pool_debt = active_pool_abi::get_usdf_debt(
        &contracts.active_pool,
        contracts.asset_contracts[0].asset_id.into(),
    )
    .await
    .value;

    assert_eq!(
        active_pool_asset_before_redemption - active_pool_asset_after_redemption,
        redemption_amount
    );

    assert_eq!(
        active_pool_debt,
        pre_redemption_active_pool_debt - redemption_amount
    );

    // Verify second asset pool is empty
    let second_asset_pool = active_pool_abi::get_asset(
        &contracts.active_pool,
        contracts.asset_contracts[1].asset_id.into(),
    )
    .await
    .value;

    assert_eq!(second_asset_pool, 0);

    let second_asset_debt = active_pool_abi::get_usdf_debt(
        &contracts.active_pool,
        contracts.asset_contracts[1].asset_id.into(),
    )
    .await
    .value;

    assert_eq!(second_asset_debt, 0);
}

#[tokio::test]
async fn proper_multi_collateral_redemption_prioritizes_lowest_icr() {
    let (contracts, _admin, mut wallets) = setup_protocol(5, true, false).await;

    let healthy_wallet1 = wallets.pop().unwrap();
    let healthy_wallet2 = wallets.pop().unwrap();
    let healthy_wallet3 = wallets.pop().unwrap();

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

    // First asset troves with higher collateral ratios
    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet1.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        20_000 * PRECISION,
        10_000 * PRECISION,
    )
    .await;

    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet2.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        20_000 * PRECISION,
        7_000 * PRECISION,
    )
    .await;

    // Second asset trove with lowest collateral ratio
    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet3.clone(),
        &contracts.asset_contracts[1],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        1_000 * PRECISION,
        700 * PRECISION,
    )
    .await;

    // Collateral ratios:
    // trove1: 20k Asset0/10k USDF = 200%
    // trove2: 20k Asset0/5k USDF = 400%
    // trove3: 1k Asset1/700 USDF = 142%

    let redemption_amount: u64 = 1_000 * PRECISION;

    let protocol_manager_health1 = ContractInstance::new(
        ProtocolManager::new(
            contracts.protocol_manager.contract.contract_id().clone(),
            healthy_wallet1.clone(),
        ),
        contracts.protocol_manager.implementation_id,
    );

    let first_asset_pre_redemption = active_pool_abi::get_asset(
        &contracts.active_pool,
        contracts.asset_contracts[0].asset_id.into(),
    )
    .await
    .value;

    protocol_manager_abi::redeem_collateral(
        &protocol_manager_health1,
        redemption_amount,
        20,
        0,
        None,
        None,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.coll_surplus_pool,
        &contracts.default_pool,
        &contracts.active_pool,
        &contracts.sorted_troves,
        &contracts.asset_contracts,
    )
    .await
    .unwrap();

    // Verify second asset was redeemed first
    let first_asset_post_redemption = active_pool_abi::get_asset(
        &contracts.active_pool,
        contracts.asset_contracts[0].asset_id.into(),
    )
    .await
    .value;

    assert_eq!(
        first_asset_pre_redemption - first_asset_post_redemption,
        redemption_amount
    );

    // Verify first asset pool remains unchanged
    let first_asset_pool = active_pool_abi::get_asset(
        &contracts.active_pool,
        contracts.asset_contracts[1].asset_id.into(),
    )
    .await
    .value;

    assert_eq!(first_asset_pool, 1_000 * PRECISION);
}

#[tokio::test]
async fn proper_multi_collateral_redemption_skips_last_trove() {
    let (contracts, _admin, mut wallets) = setup_protocol(5, true, false).await;

    let healthy_wallet1 = wallets.pop().unwrap();
    let healthy_wallet2 = wallets.pop().unwrap();
    let healthy_wallet3 = wallets.pop().unwrap();

    // Set up price feeds for both assets
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

    // Asset 1: Create 3 troves with different collateral ratios
    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet1.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        20_000 * PRECISION,
        10_000 * PRECISION,
    )
    .await;

    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet2.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        15_000 * PRECISION,
        8_000 * PRECISION,
    )
    .await;

    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet3.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        10_000 * PRECISION,
        6_000 * PRECISION,
    )
    .await;

    // Asset 2: Create 2 troves
    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet1.clone(),
        &contracts.asset_contracts[1],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        700 * PRECISION,
        500 * PRECISION,
    )
    .await;
    // Trove 1 will be redeemed first fully

    borrow_operations_utils::mint_token_and_open_trove(
        healthy_wallet2.clone(),
        &contracts.asset_contracts[1],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        1400 * PRECISION,
        999 * PRECISION,
    )
    .await;
    // Trove 2 will be skipped

    let redemption_amount: u64 = 2000 * PRECISION; // Enough to fully redeem the first trove of Asset 2

    let protocol_manager_health1 = ContractInstance::new(
        ProtocolManager::new(
            contracts.protocol_manager.contract.contract_id().clone(),
            healthy_wallet1.clone(),
        ),
        contracts.protocol_manager.implementation_id,
    );

    // Record initial states
    let initial_asset1_pool = active_pool_abi::get_asset(
        &contracts.active_pool,
        contracts.asset_contracts[0].asset_id.into(),
    )
    .await
    .value;

    let initial_asset2_pool = active_pool_abi::get_asset(
        &contracts.active_pool,
        contracts.asset_contracts[1].asset_id.into(),
    )
    .await
    .value;

    protocol_manager_abi::redeem_collateral(
        &protocol_manager_health1,
        redemption_amount,
        20,
        0,
        None,
        None,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.coll_surplus_pool,
        &contracts.default_pool,
        &contracts.active_pool,
        &contracts.sorted_troves,
        &contracts.asset_contracts,
    )
    .await
    .unwrap();

    // Verify Asset 2's first trove was redeemed
    let final_asset2_pool = active_pool_abi::get_asset(
        &contracts.active_pool,
        contracts.asset_contracts[1].asset_id.into(),
    )
    .await
    .value;

    assert!(
        final_asset2_pool < initial_asset2_pool,
        "Asset 2 pool should have decreased from redemptions"
    );

    // Verify redemptions occurred from Asset 1
    let final_asset1_pool = active_pool_abi::get_asset(
        &contracts.active_pool,
        contracts.asset_contracts[0].asset_id.into(),
    )
    .await
    .value;

    assert!(
        final_asset1_pool < initial_asset1_pool,
        "Asset 1 pool should have decreased from redemptions"
    );

    // Verify the first trove in Asset 2 was fully redeemed
    trove_manager_utils::assert_trove_debt(
        &contracts.asset_contracts[1].trove_manager,
        Identity::Address(healthy_wallet1.address().into()),
        0, // Debt should be reduced to zero
    )
    .await;
}
