use fuels::{prelude::*, types::Identity};
use test_utils::data_structures::{ContractInstance, PRECISION};
use test_utils::interfaces::oracle::oracle_abi;
use test_utils::interfaces::protocol_manager::ProtocolManager;
use test_utils::interfaces::pyth_oracle::PYTH_TIMESTAMP;
use test_utils::utils::print_response;
use test_utils::{
    interfaces::{
        active_pool::active_pool_abi,
        borrow_operations::{borrow_operations_abi, BorrowOperations},
        coll_surplus_pool::coll_surplus_pool_abi,
        protocol_manager::protocol_manager_abi,
        pyth_oracle::{pyth_oracle_abi, pyth_price_feed},
        token::token_abi,
        trove_manager::{trove_manager_abi, trove_manager_utils, Status},
    },
    setup::common::setup_protocol,
    utils::with_min_borrow_fee,
};

#[tokio::test]
async fn proper_redemption_from_partially_closed() {
    let (contracts, _admin, mut wallets) = setup_protocol(5, true, false).await;

    let healthy_wallet1 = wallets.pop().unwrap();
    let healthy_wallet2 = wallets.pop().unwrap();
    let healthy_wallet3 = wallets.pop().unwrap();

    let balance = 10_000 * PRECISION;

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        balance,
        Identity::Address(healthy_wallet1.address().into()),
    )
    .await;

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        balance,
        Identity::Address(healthy_wallet2.address().into()),
    )
    .await;

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        balance,
        Identity::Address(healthy_wallet3.address().into()),
    )
    .await;

    let borrow_operations_healthy_wallet1 = ContractInstance::new(
        BorrowOperations::new(
            contracts.borrow_operations.contract.contract_id().clone(),
            healthy_wallet1.clone(),
        ),
        contracts.borrow_operations.implementation_id.clone(),
    );

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(1),
    )
    .await;

    borrow_operations_abi::open_trove(
        &borrow_operations_healthy_wallet1,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        10_000 * PRECISION,
        5_000 * PRECISION,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    let borrow_operations_healthy_wallet2 = ContractInstance::new(
        BorrowOperations::new(
            contracts.borrow_operations.contract.contract_id().clone(),
            healthy_wallet2.clone(),
        ),
        contracts.borrow_operations.implementation_id.clone(),
    );

    borrow_operations_abi::open_trove(
        &borrow_operations_healthy_wallet2,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        9_000 * PRECISION,
        5_000 * PRECISION,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    let borrow_operations_healthy_wallet3 = ContractInstance::new(
        BorrowOperations::new(
            contracts.borrow_operations.contract.contract_id().clone(),
            healthy_wallet3.clone(),
        ),
        contracts.borrow_operations.implementation_id.clone(),
    );

    borrow_operations_abi::open_trove(
        &borrow_operations_healthy_wallet3,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        8_000 * PRECISION,
        5_000 * PRECISION,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

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
        contracts.asset_contracts[0].asset_id,
    )
    .await
    .value;

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[1].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[1].mock_pyth_oracle,
        pyth_price_feed(1),
    )
    .await;

    let res = protocol_manager_abi::redeem_collateral(
        &protocol_manager_health1,
        redemption_amount,
        10,
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

    let logs = res.decode_logs();
    let redemption_event = logs
        .results
        .iter()
        .find(|log| log.as_ref().unwrap().contains("RedemptionEvent"))
        .expect("RedemptionEvent not found")
        .as_ref()
        .unwrap();

    assert!(
        redemption_event.contains(&healthy_wallet3.address().hash().to_string()),
        "RedemptionEvent should contain user address"
    );
    assert!(
        redemption_event.contains(&redemption_amount.to_string()),
        "RedemptionEvent should contain redemption amount"
    );
    print_response(&res);

    let active_pool_asset = active_pool_abi::get_asset(
        &contracts.active_pool,
        contracts.asset_contracts[0].asset_id,
    )
    .await
    .value;

    let active_pool_debt = active_pool_abi::get_usdf_debt(
        &contracts.active_pool,
        contracts.asset_contracts[0].asset_id,
    )
    .await
    .value;

    println!("active_pool_asset: {}", active_pool_asset);
    println!("active_pool_debt: {}", active_pool_debt);
    println!(
        "pre_redemption_active_pool_debt: {}",
        pre_redemption_active_pool_debt
    );
    println!("redemption_amount: {}", redemption_amount);

    assert_eq!(
        active_pool_debt,
        pre_redemption_active_pool_debt - redemption_amount
    );

    assert_eq!(active_pool_asset, 24_000 * PRECISION);

    let provider = healthy_wallet1.provider().unwrap();

    let mock_asset_id = contracts.asset_contracts[0].asset_id;

    let mock_balance = provider
        .get_asset_balance(healthy_wallet1.address(), mock_asset_id)
        .await
        .unwrap();

    let staking_balance = provider
        .get_contract_asset_balance(&contracts.fpt_staking.contract.contract_id(), mock_asset_id)
        .await
        .unwrap();

    // here we need to calculate the fee and subtract it
    let redemption_asset_fee = trove_manager_abi::get_redemption_fee(redemption_amount);

    assert_eq!(staking_balance, redemption_asset_fee);
    assert_eq!(mock_balance, redemption_amount - redemption_asset_fee);

    trove_manager_utils::assert_trove_coll(
        &contracts.asset_contracts[0].trove_manager,
        Identity::Address(healthy_wallet3.address().into()),
        5_000 * PRECISION,
    )
    .await;

    trove_manager_utils::assert_trove_debt(
        &contracts.asset_contracts[0].trove_manager,
        Identity::Address(healthy_wallet3.address().into()),
        with_min_borrow_fee(5_000 * PRECISION) - 3_000 * PRECISION,
    )
    .await;
}

#[tokio::test]
async fn proper_redemption_with_a_trove_closed_fully() {
    let (contracts, _admin, mut wallets) = setup_protocol(5, true, false).await;

    let healthy_wallet1 = wallets.pop().unwrap();
    let healthy_wallet2 = wallets.pop().unwrap();
    let healthy_wallet3 = wallets.pop().unwrap();

    let balance: u64 = 12_000 * PRECISION;

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        balance,
        Identity::Address(healthy_wallet1.address().into()),
    )
    .await;

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        balance,
        Identity::Address(healthy_wallet2.address().into()),
    )
    .await;

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        balance,
        Identity::Address(healthy_wallet3.address().into()),
    )
    .await;

    let borrow_operations_healthy_wallet1 = ContractInstance::new(
        BorrowOperations::new(
            contracts.borrow_operations.contract.contract_id().clone(),
            healthy_wallet1.clone(),
        ),
        contracts.borrow_operations.implementation_id.clone(),
    );

    let coll1 = 12_000 * PRECISION;
    let debt1 = 6_000 * PRECISION;

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(1),
    )
    .await;

    borrow_operations_abi::open_trove(
        &borrow_operations_healthy_wallet1,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        coll1,
        debt1,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    let borrow_operations_healthy_wallet2 = ContractInstance::new(
        BorrowOperations::new(
            contracts.borrow_operations.contract.contract_id().clone(),
            healthy_wallet2.clone(),
        ),
        contracts.borrow_operations.implementation_id.clone(),
    );

    let coll2: u64 = 9_000 * PRECISION;
    let debt2: u64 = 5_000 * PRECISION;
    borrow_operations_abi::open_trove(
        &borrow_operations_healthy_wallet2,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        coll2,
        debt2,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    let borrow_operations_healthy_wallet3 = ContractInstance::new(
        BorrowOperations::new(
            contracts.borrow_operations.contract.contract_id().clone(),
            healthy_wallet3.clone(),
        ),
        contracts.borrow_operations.implementation_id.clone(),
    );

    let coll3: u64 = 8_000 * PRECISION;
    let debt3: u64 = 5_000 * PRECISION;
    borrow_operations_abi::open_trove(
        &borrow_operations_healthy_wallet3,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        coll3,
        debt3,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    // Troves
    // H1: 12/6 = 2 -> H2: 9/5 = 1.8 -> H3: 8/5 = 1.6

    let redemption_amount: u64 = 6_000 * PRECISION;

    let protocol_manager_health1 = ContractInstance::new(
        ProtocolManager::new(
            contracts.protocol_manager.contract.contract_id().clone(),
            healthy_wallet1.clone(),
        ),
        contracts.protocol_manager.implementation_id,
    );

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[1].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[1].mock_pyth_oracle,
        pyth_price_feed(1),
    )
    .await;

    protocol_manager_abi::redeem_collateral(
        &protocol_manager_health1,
        redemption_amount,
        10,
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
        contracts.asset_contracts[0].asset_id,
    )
    .await
    .value;

    let active_pool_debt = active_pool_abi::get_usdf_debt(
        &contracts.active_pool,
        contracts.asset_contracts[0].asset_id,
    )
    .await
    .value;

    let collateral_taken_from_trove3 = with_min_borrow_fee(5_000 * PRECISION);
    let remaining_collateral_to_redeem = redemption_amount - collateral_taken_from_trove3;

    assert_eq!(
        active_pool_asset,
        coll1 + coll2 - remaining_collateral_to_redeem
    );

    let total_debt = with_min_borrow_fee(debt1 + debt2 + debt3);
    assert_eq!(active_pool_debt, total_debt - redemption_amount);

    let provider = healthy_wallet1.provider().unwrap();

    let mock_asset_id = contracts.asset_contracts[0].asset_id;

    let mock_balance = provider
        .get_asset_balance(healthy_wallet1.address(), mock_asset_id)
        .await
        .unwrap();

    let staking_balance = provider
        .get_contract_asset_balance(&contracts.fpt_staking.contract.contract_id(), mock_asset_id)
        .await
        .unwrap();

    assert_eq!(mock_balance, 6_000 * PRECISION - staking_balance);

    trove_manager_utils::assert_trove_status(
        &contracts.asset_contracts[0].trove_manager,
        Identity::Address(healthy_wallet3.address().into()),
        Status::ClosedByRedemption,
    )
    .await;

    trove_manager_utils::assert_trove_coll(
        &contracts.asset_contracts[0].trove_manager,
        Identity::Address(healthy_wallet3.address().into()),
        0,
    )
    .await;

    trove_manager_utils::assert_trove_debt(
        &contracts.asset_contracts[0].trove_manager,
        Identity::Address(healthy_wallet3.address().into()),
        0,
    )
    .await;

    trove_manager_utils::assert_trove_status(
        &contracts.asset_contracts[0].trove_manager,
        Identity::Address(healthy_wallet2.address().into()),
        Status::Active,
    )
    .await;

    trove_manager_utils::assert_trove_coll(
        &contracts.asset_contracts[0].trove_manager,
        Identity::Address(healthy_wallet2.address().into()),
        9_000 * PRECISION - remaining_collateral_to_redeem,
    )
    .await;

    trove_manager_utils::assert_trove_debt(
        &contracts.asset_contracts[0].trove_manager,
        Identity::Address(healthy_wallet2.address().into()),
        with_min_borrow_fee(debt2) - remaining_collateral_to_redeem,
    )
    .await;

    let coll_surplus = coll_surplus_pool_abi::get_collateral(
        &contracts.coll_surplus_pool,
        Identity::Address(healthy_wallet3.address().into()),
        contracts.asset_contracts[0].asset_id.into(),
    )
    .await
    .unwrap()
    .value;

    assert_eq!(coll_surplus, coll3 - with_min_borrow_fee(debt3));
}

#[tokio::test]
async fn multiple_partial_redemptions_succeed() {
    let (contracts, _admin, mut wallets) = setup_protocol(5, true, false).await;

    let redeemer = wallets.pop().unwrap();
    let trove1 = wallets.pop().unwrap();

    // Mint initial balances
    let balance = 15_000 * PRECISION;
    for wallet in [&redeemer, &trove1] {
        token_abi::mint_to_id(
            &contracts.asset_contracts[0].asset,
            balance,
            Identity::Address(wallet.address().into()),
        )
        .await;
    }

    // Set up oracle prices
    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(1),
    )
    .await;

    // Open troves with different collateral ratios
    let trove_setups = [
        (&trove1, 10_000 * PRECISION, 6_000 * PRECISION), // CR = 1.4
        (&redeemer, 15_000 * PRECISION, 8_000 * PRECISION), // CR = 15
    ];

    for (wallet, coll, debt) in trove_setups {
        let borrow_ops = ContractInstance::new(
            BorrowOperations::new(
                contracts.borrow_operations.contract.contract_id().clone(),
                wallet.clone(),
            ),
            contracts.borrow_operations.implementation_id.clone(),
        );

        borrow_operations_abi::open_trove(
            &borrow_ops,
            &contracts.asset_contracts[0].oracle,
            &contracts.asset_contracts[0].mock_pyth_oracle,
            &contracts.asset_contracts[0].mock_redstone_oracle,
            &contracts.asset_contracts[0].asset,
            &contracts.usdf,
            &contracts.fpt_staking,
            &contracts.sorted_troves,
            &contracts.asset_contracts[0].trove_manager,
            &contracts.active_pool,
            coll,
            debt,
            Identity::Address(Address::zeroed()),
            Identity::Address(Address::zeroed()),
        )
        .await
        .unwrap();
    }

    // Set up protocol manager for redeemer
    let protocol_manager_redeemer = ContractInstance::new(
        ProtocolManager::new(
            contracts.protocol_manager.contract.contract_id().clone(),
            redeemer.clone(),
        ),
        contracts.protocol_manager.implementation_id,
    );

    // Perform first redemption goes into single redemption with debt below min
    let first_redemption = 5_800 * PRECISION;
    protocol_manager_abi::redeem_collateral(
        &protocol_manager_redeemer,
        first_redemption,
        10,
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

    // Perform second redemption
    let second_redemption = 200 * PRECISION;
    protocol_manager_abi::redeem_collateral(
        &protocol_manager_redeemer,
        second_redemption,
        10,
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
}
