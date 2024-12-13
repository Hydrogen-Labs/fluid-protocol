use crate::utils::setup::setup;
use fuels::{prelude::*, types::Identity};
use test_utils::{
    data_structures::{ContractInstance, PRECISION},
    interfaces::{
        borrow_operations::{borrow_operations_abi, borrow_operations_utils, BorrowOperations},
        community_issuance::community_issuance_abi,
        oracle::oracle_abi,
        pyth_oracle::{
            pyth_oracle_abi, pyth_price_feed, pyth_price_feed_with_time, PYTH_PRECISION,
            PYTH_TIMESTAMP,
        },
        stability_pool::{stability_pool_abi, stability_pool_utils, StabilityPool},
        token::token_abi,
        trove_manager::trove_manager_abi,
    },
    setup::common::{add_asset, assert_within_threshold, setup_protocol},
    utils::with_min_borrow_fee,
};

#[tokio::test]
async fn proper_initialization() {
    let (stability_pool, _, fuel, _, _, _) = setup(Some(4)).await;

    let mock_asset_id: AssetId = fuel
        .contract_id()
        .asset_id(&AssetId::zeroed().into())
        .into();

    stability_pool_utils::assert_pool_asset(&stability_pool, 0, mock_asset_id).await;

    stability_pool_utils::assert_total_usdf_deposits(&stability_pool, 0).await;
}

#[tokio::test]
async fn proper_stability_deposit() {
    let (contracts, admin, _wallets) = setup_protocol(4, false, false).await;

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(1),
    )
    .await;

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
        1_200 * PRECISION,
        600 * PRECISION,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();
    let deposit_amount = 600 * PRECISION;
    let res = stability_pool_abi::provide_to_stability_pool(
        &contracts.stability_pool,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        deposit_amount,
    )
    .await
    .unwrap();

    let logs = res.decode_logs();
    let provide_event = logs
        .results
        .iter()
        .find(|log| {
            log.as_ref()
                .unwrap()
                .contains("ProvideToStabilityPoolEvent")
        })
        .expect("ProvideToStabilityPoolEvent not found")
        .as_ref()
        .unwrap();

    assert!(
        provide_event.contains(&admin.address().hash().to_string()),
        "ProvideToStabilityPoolEvent should contain user address"
    );
    assert!(
        provide_event.contains(&deposit_amount.to_string()),
        "ProvideToStabilityPoolEvent should contain deposit amount"
    );

    // print_response(&res);

    stability_pool_utils::assert_pool_asset(
        &contracts.stability_pool,
        0,
        contracts.asset_contracts[0].asset_id.into(),
    )
    .await;

    stability_pool_utils::assert_total_usdf_deposits(&contracts.stability_pool, 600 * PRECISION)
        .await;

    stability_pool_utils::assert_compounded_usdf_deposit(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        600 * PRECISION,
    )
    .await;

    stability_pool_utils::assert_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        0,
        contracts.asset_contracts[0].asset_id,
    )
    .await;
}

#[tokio::test]
async fn proper_stability_widthdrawl() {
    let (contracts, admin, _wallets) = setup_protocol(4, false, false).await;

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(1),
    )
    .await;

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
        1_200 * PRECISION,
        600 * PRECISION,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    stability_pool_abi::provide_to_stability_pool(
        &contracts.stability_pool,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        600 * PRECISION,
    )
    .await
    .unwrap();
    let withdraw_amount = 300 * PRECISION;
    let res = stability_pool_abi::withdraw_from_stability_pool(
        &contracts.stability_pool,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].trove_manager,
        withdraw_amount,
    )
    .await
    .unwrap();

    let logs = res.decode_logs();
    let withdraw_event = logs
        .results
        .iter()
        .find(|log| {
            log.as_ref()
                .unwrap()
                .contains("WithdrawFromStabilityPoolEvent")
        })
        .expect("WithdrawFromStabilityPoolEvent not found")
        .as_ref()
        .unwrap();

    assert!(
        withdraw_event.contains(&admin.address().hash().to_string()),
        "WithdrawFromStabilityPoolEvent should contain user address"
    );
    assert!(
        withdraw_event.contains(&withdraw_amount.to_string()),
        "WithdrawFromStabilityPoolEvent should contain withdraw amount"
    );

    stability_pool_utils::assert_pool_asset(
        &contracts.stability_pool,
        0,
        contracts.asset_contracts[0].asset_id.into(),
    )
    .await;

    stability_pool_utils::assert_total_usdf_deposits(&contracts.stability_pool, withdraw_amount)
        .await;

    stability_pool_utils::assert_compounded_usdf_deposit(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        withdraw_amount,
    )
    .await;

    stability_pool_utils::assert_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        0,
        contracts.asset_contracts[0].asset_id.into(),
    )
    .await;
}

#[tokio::test]
async fn proper_one_sp_depositor_position() {
    let (contracts, admin, mut wallets) = setup_protocol(4, false, false).await;

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(10),
    )
    .await;

    let liquidated_wallet = wallets.pop().unwrap();

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        6_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        5_000 * PRECISION,
        Identity::Address(liquidated_wallet.address().into()),
    )
    .await;

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
        6_000 * PRECISION,
        3_000 * PRECISION,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    let liq_borrow_operations = ContractInstance::new(
        BorrowOperations::new(
            contracts.borrow_operations.contract.contract_id().clone(),
            liquidated_wallet.clone(),
        ),
        contracts.borrow_operations.implementation_id.clone(),
    );

    borrow_operations_abi::open_trove(
        &liq_borrow_operations,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        1_100 * PRECISION,
        1_000 * PRECISION,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    let init_stability_deposit = 1_500 * PRECISION;
    stability_pool_abi::provide_to_stability_pool(
        &contracts.stability_pool,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        init_stability_deposit,
    )
    .await
    .unwrap();

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP + 1).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed_with_time(1, PYTH_TIMESTAMP + 1, PYTH_PRECISION.into()),
    )
    .await;

    trove_manager_abi::liquidate(
        &contracts.asset_contracts[0].trove_manager,
        &contracts.community_issuance,
        &contracts.stability_pool,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.sorted_troves,
        &contracts.active_pool,
        &contracts.default_pool,
        &contracts.coll_surplus_pool,
        &contracts.usdf,
        Identity::Address(liquidated_wallet.address().into()),
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    // Since the entire debt is liquidated including the borrow fee,
    // the asset recieved includes the 0.5% fee
    let mut asset_with_fee_adjustment = 1_100 * PRECISION;
    let gas_coll_fee = asset_with_fee_adjustment / 200;
    asset_with_fee_adjustment -= gas_coll_fee;
    let debt_with_fee_adjustment = with_min_borrow_fee(1_000 * PRECISION);

    stability_pool_utils::assert_pool_asset(
        &contracts.stability_pool,
        asset_with_fee_adjustment,
        contracts.asset_contracts[0].asset_id,
    )
    .await;

    stability_pool_utils::assert_total_usdf_deposits(
        &contracts.stability_pool,
        init_stability_deposit - debt_with_fee_adjustment,
    )
    .await;

    stability_pool_utils::assert_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        asset_with_fee_adjustment,
        contracts.asset_contracts[0].asset_id,
    )
    .await;

    // 500 - 0.5% fee
    stability_pool_utils::assert_compounded_usdf_deposit(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        init_stability_deposit - debt_with_fee_adjustment,
    )
    .await;

    // Makes a 2nd deposit to the Stability Pool
    let second_deposit = 1_000 * PRECISION;

    stability_pool_abi::provide_to_stability_pool(
        &contracts.stability_pool,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        second_deposit,
    )
    .await
    .unwrap();

    stability_pool_utils::assert_compounded_usdf_deposit(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        init_stability_deposit - debt_with_fee_adjustment + second_deposit,
    )
    .await;

    // Gain has been withdrawn and resset
    stability_pool_utils::assert_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        0,
        contracts.asset_contracts[0].asset_id,
    )
    .await;

    let provider = admin.provider().unwrap();

    let mock_asset_id: AssetId = contracts.asset_contracts[0].asset_id;

    let mock_balance = provider
        .get_asset_balance(admin.address(), mock_asset_id)
        .await
        .unwrap();

    assert_within_threshold(
        mock_balance,
        asset_with_fee_adjustment + gas_coll_fee,
        "Mock balance is not correct",
    )
}

#[tokio::test]
async fn proper_many_depositors_distribution() {
    let (contracts, admin, mut wallets) = setup_protocol(4, false, false).await;

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(10),
    )
    .await;

    let liquidated_wallet = wallets.pop().unwrap();
    let depositor_2 = wallets.pop().unwrap();
    let depositor_3 = wallets.pop().unwrap();

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        6_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        5_000 * PRECISION,
        Identity::Address(liquidated_wallet.address().into()),
    )
    .await;

    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(10),
    )
    .await;

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
        6_000 * PRECISION,
        3_000 * PRECISION,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    let liq_borrow_operations = ContractInstance::new(
        BorrowOperations::new(
            contracts.borrow_operations.contract.contract_id().clone(),
            liquidated_wallet.clone(),
        ),
        contracts.borrow_operations.implementation_id.clone(),
    );

    borrow_operations_abi::open_trove(
        &liq_borrow_operations,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        1_100 * PRECISION,
        1_000 * PRECISION,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    stability_pool_abi::provide_to_stability_pool(
        &contracts.stability_pool,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        2_000 * PRECISION,
    )
    .await
    .unwrap();

    let usdf_asset_id: AssetId = contracts.usdf_asset_id;
    let tx_params = TxPolicies::default().with_tip(1);

    admin
        .transfer(
            depositor_2.address().into(),
            500 * PRECISION,
            usdf_asset_id,
            tx_params,
        )
        .await
        .unwrap();

    admin
        .transfer(
            depositor_3.address().into(),
            500 * PRECISION,
            usdf_asset_id,
            tx_params,
        )
        .await
        .unwrap();

    let depositor_2_sp = ContractInstance::new(
        StabilityPool::new(
            contracts.stability_pool.contract.contract_id().clone(),
            depositor_2.clone(),
        ),
        contracts.stability_pool.implementation_id,
    );

    let depositor_3_sp = ContractInstance::new(
        StabilityPool::new(
            contracts.stability_pool.contract.contract_id().clone(),
            depositor_3.clone(),
        ),
        contracts.stability_pool.implementation_id,
    );

    stability_pool_abi::provide_to_stability_pool(
        &depositor_2_sp,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        500 * PRECISION,
    )
    .await
    .unwrap();

    stability_pool_abi::provide_to_stability_pool(
        &depositor_3_sp,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        500 * PRECISION,
    )
    .await
    .unwrap();

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP + 1).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed_with_time(1, PYTH_TIMESTAMP + 1, PYTH_PRECISION.into()),
    )
    .await;

    trove_manager_abi::liquidate(
        &contracts.asset_contracts[0].trove_manager,
        &contracts.community_issuance,
        &contracts.stability_pool,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.sorted_troves,
        &contracts.active_pool,
        &contracts.default_pool,
        &contracts.coll_surplus_pool,
        &contracts.usdf,
        Identity::Address(liquidated_wallet.address().into()),
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    // Fully liquidated
    let mut asset_with_fee_adjustment = 1_100 * PRECISION;
    let gas_coll_fee = asset_with_fee_adjustment / 200;
    asset_with_fee_adjustment -= gas_coll_fee;
    let debt_paid_off = with_min_borrow_fee(1_000 * PRECISION);

    stability_pool_utils::assert_pool_asset(
        &contracts.stability_pool,
        asset_with_fee_adjustment,
        contracts.asset_contracts[0].asset_id,
    )
    .await;

    // 3,000 initially deposited, 1000 used to pay off debt, 1,500 left in pool
    stability_pool_utils::assert_total_usdf_deposits(
        &contracts.stability_pool,
        3_000 * PRECISION - debt_paid_off,
    )
    .await;

    // Admin is 2/3 of the pool, depositor_2 is 1/6, depositor_3 is 1/6
    stability_pool_utils::assert_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        asset_with_fee_adjustment * 2 / 3,
        contracts.asset_contracts[0].asset_id,
    )
    .await;

    // Admin lost 2/3 of the usdf used to pay off debt
    stability_pool_utils::assert_compounded_usdf_deposit(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        2_000 * PRECISION - debt_paid_off * 2 / 3,
    )
    .await;

    stability_pool_utils::assert_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(depositor_2.address().into()),
        asset_with_fee_adjustment / 6,
        contracts.asset_contracts[0].asset_id,
    )
    .await;

    stability_pool_utils::assert_compounded_usdf_deposit(
        &contracts.stability_pool,
        Identity::Address(depositor_2.address().into()),
        500 * PRECISION - debt_paid_off / 6,
    )
    .await;
}

#[tokio::test]
async fn proper_no_reward_when_depositing_and_rewards_already_distributed() {
    let (contracts, admin, mut wallets) = setup_protocol(4, false, false).await;

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(10),
    )
    .await;

    let liquidated_wallet = wallets.pop().unwrap();
    let depositor_2 = wallets.pop().unwrap();

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        6_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        5_000 * PRECISION,
        Identity::Address(liquidated_wallet.address().into()),
    )
    .await;

    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(10),
    )
    .await;

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
        6_000 * PRECISION,
        3_000 * PRECISION,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    let liq_borrow_operations = ContractInstance::new(
        BorrowOperations::new(
            contracts.borrow_operations.contract.contract_id().clone(),
            liquidated_wallet.clone(),
        ),
        contracts.borrow_operations.implementation_id.clone(),
    );

    borrow_operations_abi::open_trove(
        &liq_borrow_operations,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.active_pool,
        1_100 * PRECISION,
        1_000 * PRECISION,
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    stability_pool_abi::provide_to_stability_pool(
        &contracts.stability_pool,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        2_000 * PRECISION,
    )
    .await
    .unwrap();

    let usdf_asset_id: AssetId = contracts.usdf_asset_id;
    let tx_params = TxPolicies::default().with_tip(1);

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP + 1).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed_with_time(1, PYTH_TIMESTAMP + 1, PYTH_PRECISION.into()),
    )
    .await;

    trove_manager_abi::liquidate(
        &contracts.asset_contracts[0].trove_manager,
        &contracts.community_issuance,
        &contracts.stability_pool,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.sorted_troves,
        &contracts.active_pool,
        &contracts.default_pool,
        &contracts.coll_surplus_pool,
        &contracts.usdf,
        Identity::Address(liquidated_wallet.address().into()),
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    admin
        .transfer(
            depositor_2.address().into(),
            500 * PRECISION,
            usdf_asset_id,
            tx_params,
        )
        .await
        .unwrap();

    let depositor_2_sp = ContractInstance::new(
        StabilityPool::new(
            contracts.stability_pool.contract.contract_id().clone(),
            depositor_2.clone(),
        ),
        contracts.stability_pool.implementation_id,
    );

    stability_pool_abi::provide_to_stability_pool(
        &depositor_2_sp,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        500 * PRECISION,
    )
    .await
    .unwrap();

    stability_pool_utils::assert_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(depositor_2.address().into()),
        0,
        contracts.asset_contracts[0].asset_id,
    )
    .await;

    stability_pool_utils::assert_compounded_usdf_deposit(
        &contracts.stability_pool,
        Identity::Address(depositor_2.address().into()),
        500 * PRECISION,
    )
    .await;
}

#[tokio::test]
async fn proper_one_sp_depositor_position_multiple_assets() {
    let (contracts, admin, mut wallets) = setup_protocol(4, true, false).await;

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(10),
    )
    .await;

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[1].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[1].mock_pyth_oracle,
        pyth_price_feed(10),
    )
    .await;

    let liquidated_wallet = wallets.pop().unwrap();

    borrow_operations_utils::mint_token_and_open_trove(
        admin.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        6_000 * PRECISION,
        3_000 * PRECISION,
    )
    .await;

    borrow_operations_utils::mint_token_and_open_trove(
        liquidated_wallet.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        1_100 * PRECISION,
        1_000 * PRECISION,
    )
    .await;

    borrow_operations_utils::mint_token_and_open_trove(
        admin.clone(),
        &contracts.asset_contracts[1],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        6_000 * PRECISION,
        3_000 * PRECISION,
    )
    .await;

    borrow_operations_utils::mint_token_and_open_trove(
        liquidated_wallet.clone(),
        &contracts.asset_contracts[1],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        1_100 * PRECISION,
        1_000 * PRECISION,
    )
    .await;

    let init_stability_deposit = 3_000 * PRECISION;
    stability_pool_abi::provide_to_stability_pool(
        &contracts.stability_pool,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        init_stability_deposit,
    )
    .await
    .unwrap();

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP + 1).await;
    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[1].oracle, PYTH_TIMESTAMP + 1).await;

    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed_with_time(1, PYTH_TIMESTAMP + 1, PYTH_PRECISION.into()),
    )
    .await;

    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[1].mock_pyth_oracle,
        pyth_price_feed_with_time(1, PYTH_TIMESTAMP + 1, PYTH_PRECISION.into()),
    )
    .await;

    let res = trove_manager_abi::liquidate(
        &contracts.asset_contracts[0].trove_manager,
        &contracts.community_issuance,
        &contracts.stability_pool,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.sorted_troves,
        &contracts.active_pool,
        &contracts.default_pool,
        &contracts.coll_surplus_pool,
        &contracts.usdf,
        Identity::Address(liquidated_wallet.address().into()),
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    let logs = res.decode_logs();
    let liquidation_event = logs
        .results
        .iter()
        .find(|log| {
            log.as_ref()
                .unwrap()
                .contains("StabilityPoolLiquidationEvent")
        })
        .expect("StabilityPoolLiquidationEvent not found")
        .as_ref()
        .unwrap();

    assert!(
        liquidation_event.contains(&contracts.asset_contracts[0].asset_id.to_string()),
        "StabilityPoolLiquidationEvent should contain asset_id"
    );
    println!("liquidation_event: {}", liquidation_event);

    trove_manager_abi::liquidate(
        &contracts.asset_contracts[1].trove_manager,
        &contracts.community_issuance,
        &contracts.stability_pool,
        &contracts.asset_contracts[1].oracle,
        &contracts.asset_contracts[1].mock_pyth_oracle,
        &contracts.asset_contracts[1].mock_redstone_oracle,
        &contracts.sorted_troves,
        &contracts.active_pool,
        &contracts.default_pool,
        &contracts.coll_surplus_pool,
        &contracts.usdf,
        Identity::Address(liquidated_wallet.address().into()),
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    // Since the entire debt is liquidated including the borrow fee,
    // the asset recieved includes the 0.5% fee
    let mut asset_with_fee_adjustment = 1_100 * PRECISION;
    let coll_gas_compensation = asset_with_fee_adjustment / 200;
    asset_with_fee_adjustment -= coll_gas_compensation;
    let debt_with_fee_adjustment = with_min_borrow_fee(1_000 * PRECISION);

    stability_pool_utils::assert_pool_asset(
        &contracts.stability_pool,
        asset_with_fee_adjustment,
        contracts.asset_contracts[0].asset_id,
    )
    .await;

    stability_pool_utils::assert_total_usdf_deposits(
        &contracts.stability_pool,
        init_stability_deposit - 2 * debt_with_fee_adjustment,
    )
    .await;

    stability_pool_utils::assert_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        asset_with_fee_adjustment,
        contracts.asset_contracts[0].asset_id,
    )
    .await;

    stability_pool_utils::assert_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        asset_with_fee_adjustment,
        contracts.asset_contracts[1].asset_id,
    )
    .await;

    // 500 - 0.5% fee
    stability_pool_utils::assert_compounded_usdf_deposit(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        init_stability_deposit - 2 * debt_with_fee_adjustment,
    )
    .await;

    // Makes a 2nd deposit to the Stability Pool
    let second_deposit = 1_000 * PRECISION;

    stability_pool_abi::provide_to_stability_pool(
        &contracts.stability_pool,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        second_deposit,
    )
    .await
    .unwrap();

    stability_pool_utils::assert_compounded_usdf_deposit(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        init_stability_deposit - 2 * debt_with_fee_adjustment + second_deposit,
    )
    .await;

    // Gain has been withdrawn and resset
    stability_pool_utils::assert_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        0,
        contracts.asset_contracts[0].asset_id,
    )
    .await;

    stability_pool_utils::assert_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        0,
        contracts.asset_contracts[1].asset_id,
    )
    .await;

    let provider = admin.provider().unwrap();

    let mock_asset_id: AssetId = contracts.asset_contracts[0].asset_id;

    let mock_balance = provider
        .get_asset_balance(admin.address(), mock_asset_id)
        .await
        .unwrap();

    assert_within_threshold(
        mock_balance,
        asset_with_fee_adjustment + coll_gas_compensation,
        "Mock balance is not correct",
    );

    let provider = admin.provider().unwrap();

    let st_mock_asset_id: AssetId = contracts.asset_contracts[1].asset_id;

    let st_mock_balance = provider
        .get_asset_balance(admin.address(), st_mock_asset_id)
        .await
        .unwrap();

    assert_within_threshold(
        st_mock_balance,
        asset_with_fee_adjustment + coll_gas_compensation,
        "st_mock_balance not correct",
    );
}

#[tokio::test]
async fn proper_one_sp_depositor_position_new_asset_onboarded_midway() {
    let (mut contracts, admin, mut wallets) = setup_protocol(4, false, false).await;

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(10),
    )
    .await;

    let liquidated_wallet = wallets.pop().unwrap();

    borrow_operations_utils::mint_token_and_open_trove(
        admin.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        6_000 * PRECISION,
        3_000 * PRECISION,
    )
    .await;

    borrow_operations_utils::mint_token_and_open_trove(
        liquidated_wallet.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        1_100 * PRECISION,
        1_000 * PRECISION,
    )
    .await;

    let init_stability_deposit = 3_000 * PRECISION;
    stability_pool_abi::provide_to_stability_pool(
        &contracts.stability_pool,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        init_stability_deposit,
    )
    .await
    .unwrap();

    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP + 1).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed_with_time(1, PYTH_TIMESTAMP + 1, PYTH_PRECISION.into()),
    )
    .await;

    trove_manager_abi::liquidate(
        &contracts.asset_contracts[0].trove_manager,
        &contracts.community_issuance,
        &contracts.stability_pool,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].mock_pyth_oracle,
        &contracts.asset_contracts[0].mock_redstone_oracle,
        &contracts.sorted_troves,
        &contracts.active_pool,
        &contracts.default_pool,
        &contracts.coll_surplus_pool,
        &contracts.usdf,
        Identity::Address(liquidated_wallet.address().into()),
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    // Onboard a new asset and try to do all the same operations same as if
    // the asset was already onboarded like the prior tests

    let new_asset_contracts = add_asset(
        &mut contracts,
        &admin,
        "mock2".to_string(),
        "mock2".to_string(),
    )
    .await;

    oracle_abi::set_debug_timestamp(&new_asset_contracts.oracle, PYTH_TIMESTAMP).await;

    pyth_oracle_abi::update_price_feeds(
        &new_asset_contracts.mock_pyth_oracle,
        pyth_price_feed_with_time(10, PYTH_TIMESTAMP, PYTH_PRECISION.into()),
    )
    .await;

    borrow_operations_utils::mint_token_and_open_trove(
        admin.clone(),
        &new_asset_contracts,
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        6_000 * PRECISION,
        3_000 * PRECISION,
    )
    .await;

    borrow_operations_utils::mint_token_and_open_trove(
        liquidated_wallet.clone(),
        &new_asset_contracts,
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        1_100 * PRECISION,
        1_000 * PRECISION,
    )
    .await;

    oracle_abi::set_debug_timestamp(&new_asset_contracts.oracle, PYTH_TIMESTAMP + 1).await;
    pyth_oracle_abi::update_price_feeds(
        &new_asset_contracts.mock_pyth_oracle,
        pyth_price_feed_with_time(1, PYTH_TIMESTAMP + 1, PYTH_PRECISION.into()),
    )
    .await;

    trove_manager_abi::liquidate(
        &new_asset_contracts.trove_manager,
        &contracts.community_issuance,
        &contracts.stability_pool,
        &new_asset_contracts.oracle,
        &new_asset_contracts.mock_pyth_oracle,
        &new_asset_contracts.mock_redstone_oracle,
        &contracts.sorted_troves,
        &contracts.active_pool,
        &contracts.default_pool,
        &contracts.coll_surplus_pool,
        &contracts.usdf,
        Identity::Address(liquidated_wallet.address().into()),
        Identity::Address(Address::zeroed()),
        Identity::Address(Address::zeroed()),
    )
    .await
    .unwrap();

    // Full liquidation
    let mut asset_with_fee_adjustment = 1_100 * PRECISION;
    let gas_coll_compensation = asset_with_fee_adjustment / 200;
    asset_with_fee_adjustment -= gas_coll_compensation;
    let debt_with_fee_adjustment = with_min_borrow_fee(1_000 * PRECISION);

    stability_pool_utils::assert_pool_asset(
        &contracts.stability_pool,
        asset_with_fee_adjustment,
        contracts.asset_contracts[0].asset_id,
    )
    .await;

    stability_pool_utils::assert_total_usdf_deposits(
        &contracts.stability_pool,
        init_stability_deposit - 2 * debt_with_fee_adjustment,
    )
    .await;

    stability_pool_utils::assert_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        asset_with_fee_adjustment,
        contracts.asset_contracts[0].asset_id,
    )
    .await;

    stability_pool_utils::assert_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        asset_with_fee_adjustment,
        new_asset_contracts.asset_id,
    )
    .await;

    // 500 - 0.5% fee
    stability_pool_utils::assert_compounded_usdf_deposit(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        init_stability_deposit - 2 * debt_with_fee_adjustment,
    )
    .await;

    // Makes a 2nd deposit to the Stability Pool
    let second_deposit = 1_000 * PRECISION;

    stability_pool_abi::provide_to_stability_pool(
        &contracts.stability_pool,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        second_deposit,
    )
    .await
    .unwrap();

    stability_pool_utils::assert_compounded_usdf_deposit(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        init_stability_deposit - 2 * debt_with_fee_adjustment + second_deposit,
    )
    .await;

    // Gain has been withdrawn and resset
    stability_pool_utils::assert_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        0,
        contracts.asset_contracts[0].asset_id,
    )
    .await;

    stability_pool_utils::assert_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(admin.address().into()),
        0,
        new_asset_contracts.asset_id,
    )
    .await;

    let provider = admin.provider().unwrap();

    let mock_asset_id: AssetId = contracts.asset_contracts[0].asset_id;

    let mock_balance = provider
        .get_asset_balance(admin.address(), mock_asset_id)
        .await
        .unwrap();

    assert_within_threshold(
        mock_balance,
        asset_with_fee_adjustment + gas_coll_compensation,
        "Mock balance is not correct",
    );

    let provider = admin.provider().unwrap();

    let st_mock_asset_id: AssetId = new_asset_contracts.asset_id;

    let st_mock_balance = provider
        .get_asset_balance(admin.address(), st_mock_asset_id)
        .await
        .unwrap();

    assert_within_threshold(
        st_mock_balance,
        asset_with_fee_adjustment + gas_coll_compensation,
        "st_mock_balance not currect",
    );
}

#[tokio::test]
async fn proper_multiple_liquidations_maintain_ratio() {
    let (contracts, admin, mut wallets) = setup_protocol(20, false, false).await;
    let depositor_1 = wallets.pop().unwrap();
    let depositor_2 = wallets.pop().unwrap();
    let depositor_3 = wallets.pop().unwrap();

    // Initial setup for price feeds
    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(10),
    )
    .await;
    // Set up admin's initial trove and USDF
    borrow_operations_utils::mint_token_and_open_trove(
        admin.clone(),
        &contracts.asset_contracts[0],
        &contracts.borrow_operations,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.active_pool,
        &contracts.sorted_troves,
        100_000 * PRECISION,
        50_000 * PRECISION,
    )
    .await;

    // Transfer USDF to second depositor
    let tx_params = TxPolicies::default().with_tip(1);
    admin
        .transfer(
            depositor_2.address().into(),
            20_000 * PRECISION,
            contracts.usdf_asset_id,
            tx_params,
        )
        .await
        .unwrap();

    admin
        .transfer(
            depositor_1.address().into(),
            10_000 * PRECISION,
            contracts.usdf_asset_id,
            tx_params,
        )
        .await
        .unwrap();

    admin
        .transfer(
            depositor_3.address().into(),
            10_000 * PRECISION,
            contracts.usdf_asset_id,
            tx_params,
        )
        .await
        .unwrap();

    let depositor_1_sp = ContractInstance::new(
        StabilityPool::new(
            contracts.stability_pool.contract.contract_id().clone(),
            depositor_1.clone(),
        ),
        contracts.stability_pool.implementation_id,
    );

    // Initial deposits and shares:
    // Depositor 1: 10,000 USDF (33.33%)
    // Depositor 2: 20,000 USDF (66.67%)
    // Total Pool: 30,000 USDF
    // Make initial deposits to stability pool
    stability_pool_abi::provide_to_stability_pool(
        &depositor_1_sp,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        10_000 * PRECISION, // Admin deposits 10_000
    )
    .await
    .unwrap();

    let depositor_2_sp = ContractInstance::new(
        StabilityPool::new(
            contracts.stability_pool.contract.contract_id().clone(),
            depositor_2.clone(),
        ),
        contracts.stability_pool.implementation_id,
    );

    stability_pool_abi::provide_to_stability_pool(
        &depositor_2_sp,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        20_000 * PRECISION, // Depositor 2 deposits 20_000
    )
    .await
    .unwrap();

    // First round of liquidations (5 troves):
    // Each trove: 500 USDF debt + fee
    // Total liquidated: ~2,500 USDF
    // Expected remaining deposits after liquidations:
    // Depositor 1: ~9,167 USDF (33.33%)
    // Depositor 2: ~18,333 USDF (66.67%)
    // Total Pool: ~27,500 USDF
    // Perform 5 cycles of trove creation and liquidation
    let num_liquidations: u64 = 5;
    for i in 0..num_liquidations {
        let liquidated_wallet = wallets[i as usize].clone();
        let base_timestamp = PYTH_TIMESTAMP + (i * 2) as u64; // Each iteration gets its own timestamp window

        // Create trove to be liquidated
        borrow_operations_utils::mint_token_and_open_trove(
            liquidated_wallet.clone(),
            &contracts.asset_contracts[0],
            &contracts.borrow_operations,
            &contracts.usdf,
            &contracts.fpt_staking,
            &contracts.active_pool,
            &contracts.sorted_troves,
            550 * PRECISION, // Collateral
            500 * PRECISION, // Debt
        )
        .await;

        // Update price to trigger liquidation
        oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, base_timestamp).await;
        pyth_oracle_abi::update_price_feeds(
            &contracts.asset_contracts[0].mock_pyth_oracle,
            pyth_price_feed_with_time(1, base_timestamp, PYTH_PRECISION.into()),
        )
        .await;

        // Liquidate trove
        trove_manager_abi::liquidate(
            &contracts.asset_contracts[0].trove_manager,
            &contracts.community_issuance,
            &contracts.stability_pool,
            &contracts.asset_contracts[0].oracle,
            &contracts.asset_contracts[0].mock_pyth_oracle,
            &contracts.asset_contracts[0].mock_redstone_oracle,
            &contracts.sorted_troves,
            &contracts.active_pool,
            &contracts.default_pool,
            &contracts.coll_surplus_pool,
            &contracts.usdf,
            Identity::Address(liquidated_wallet.address().into()),
            Identity::Address(Address::zeroed()),
            Identity::Address(Address::zeroed()),
        )
        .await
        .unwrap();

        // set price high again to open next trove with a newer timestamp
        oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, base_timestamp + 1)
            .await;
        pyth_oracle_abi::update_price_feeds(
            &contracts.asset_contracts[0].mock_pyth_oracle,
            pyth_price_feed_with_time(10, base_timestamp + 1, PYTH_PRECISION.into()),
        )
        .await;
    }

    // Calculate expected total liquidated amounts
    let single_liquidation_coll = 550 * PRECISION;
    let single_liquidation_debt = with_min_borrow_fee(500 * PRECISION);
    let total_liquidated_coll = single_liquidation_coll * num_liquidations;
    let total_liquidated_debt = single_liquidation_debt * num_liquidations;
    let gas_compensation = total_liquidated_coll / 200; // 0.5%
    let actual_liquidated_coll = total_liquidated_coll - gas_compensation;

    // Verify final balances maintain 1:2 ratio
    let depositor_1_deposit = stability_pool_abi::get_compounded_usdf_deposit(
        &depositor_1_sp,
        Identity::Address(depositor_1.address().into()),
    )
    .await
    .unwrap();

    let depositor_2_deposit = stability_pool_abi::get_compounded_usdf_deposit(
        &contracts.stability_pool,
        Identity::Address(depositor_2.address().into()),
    )
    .await
    .unwrap();

    // Check USDF deposits maintain ratio (allowing for small rounding errors)
    assert_within_threshold(
        depositor_1_deposit.value * 2,
        depositor_2_deposit.value,
        "USDF deposits should maintain 1:2 ratio",
    );

    // Check asset gains maintain ratio
    let depositor_1_asset_gain = stability_pool_abi::get_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(depositor_1.address().into()),
        contracts.asset_contracts[0].asset_id,
    )
    .await
    .unwrap();

    let depositor_2_asset_gain = stability_pool_abi::get_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(depositor_2.address().into()),
        contracts.asset_contracts[0].asset_id,
    )
    .await
    .unwrap();

    assert_within_threshold(
        depositor_1_asset_gain.value * 2,
        depositor_2_asset_gain.value,
        "Asset gains should maintain 1:2 ratio",
    );

    // Verify total amounts
    let expected_remaining_deposit = 30_000 * PRECISION - total_liquidated_debt;
    stability_pool_utils::assert_total_usdf_deposits(
        &contracts.stability_pool,
        expected_remaining_deposit,
    )
    .await;

    stability_pool_utils::assert_pool_asset(
        &contracts.stability_pool,
        actual_liquidated_coll,
        contracts.asset_contracts[0].asset_id,
    )
    .await;

    let depositor_3_sp = ContractInstance::new(
        StabilityPool::new(
            contracts.stability_pool.contract.contract_id().clone(),
            depositor_3.clone(),
        ),
        contracts.stability_pool.implementation_id,
    );
    // Depositor 3 joins with same deposit as Depositor 1
    // New distribution:
    // Depositor 1: ~9,167 USDF (25%)
    // Depositor 2: ~18,333 USDF (50%)
    // Depositor 3: ~9,167 USDF (25%)
    // Total Pool: ~36,667 USDF
    stability_pool_abi::provide_to_stability_pool(
        &depositor_3_sp,
        &contracts.community_issuance,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        depositor_1_deposit.value,
    )
    .await
    .unwrap();

    // Second round of liquidations (5 more troves):
    // Each trove: 500 USDF debt + fee
    // Total new liquidated: ~2,500 USDF
    // Expected final deposits:
    // Depositor 1: ~8,542 USDF (25%)
    // Depositor 2: ~17,083 USDF (50%)
    // Depositor 3: ~8,542 USDF (25%)
    // Total Pool: ~34,167 USDF
    let num_liquidations: u64 = 5;
    for i in 5..num_liquidations + 5 {
        let liquidated_wallet = wallets[i as usize].clone();
        let base_timestamp = PYTH_TIMESTAMP + (i * 2) as u64; // Each iteration gets its own timestamp window

        // Create trove to be liquidated
        borrow_operations_utils::mint_token_and_open_trove(
            liquidated_wallet.clone(),
            &contracts.asset_contracts[0],
            &contracts.borrow_operations,
            &contracts.usdf,
            &contracts.fpt_staking,
            &contracts.active_pool,
            &contracts.sorted_troves,
            550 * PRECISION, // Collateral
            500 * PRECISION, // Debt
        )
        .await;

        // Update price to trigger liquidation
        oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, base_timestamp).await;
        pyth_oracle_abi::update_price_feeds(
            &contracts.asset_contracts[0].mock_pyth_oracle,
            pyth_price_feed_with_time(1, base_timestamp, PYTH_PRECISION.into()),
        )
        .await;

        // Liquidate trove
        trove_manager_abi::liquidate(
            &contracts.asset_contracts[0].trove_manager,
            &contracts.community_issuance,
            &contracts.stability_pool,
            &contracts.asset_contracts[0].oracle,
            &contracts.asset_contracts[0].mock_pyth_oracle,
            &contracts.asset_contracts[0].mock_redstone_oracle,
            &contracts.sorted_troves,
            &contracts.active_pool,
            &contracts.default_pool,
            &contracts.coll_surplus_pool,
            &contracts.usdf,
            Identity::Address(liquidated_wallet.address().into()),
            Identity::Address(Address::zeroed()),
            Identity::Address(Address::zeroed()),
        )
        .await
        .unwrap();

        // set price high again to open next trove with a newer timestamp
        oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, base_timestamp + 1)
            .await;
        pyth_oracle_abi::update_price_feeds(
            &contracts.asset_contracts[0].mock_pyth_oracle,
            pyth_price_feed_with_time(10, base_timestamp + 1, PYTH_PRECISION.into()),
        )
        .await;
    }

    // After second set of liquidations, verify final ratios between all depositors
    let depositor_1_final_deposit = stability_pool_abi::get_compounded_usdf_deposit(
        &depositor_1_sp,
        Identity::Address(depositor_1.address().into()),
    )
    .await
    .unwrap();

    let depositor_2_final_deposit = stability_pool_abi::get_compounded_usdf_deposit(
        &depositor_2_sp,
        Identity::Address(depositor_2.address().into()),
    )
    .await
    .unwrap();

    let depositor_3_final_deposit = stability_pool_abi::get_compounded_usdf_deposit(
        &depositor_3_sp,
        Identity::Address(depositor_3.address().into()),
    )
    .await
    .unwrap();

    // Verify 2:1:1 ratio between depositors
    assert_within_threshold(
        depositor_1_final_deposit.value * 2,
        depositor_2_final_deposit.value,
        "Depositor 1 and 2 should maintain 1:2 ratio after second liquidation",
    );

    assert_within_threshold(
        depositor_1_final_deposit.value,
        depositor_3_final_deposit.value,
        "Depositor 1 and 3 should have equal deposits after second liquidation",
    );
    // Asset gains should follow the same proportions as deposits
    // Depositor 1: 25% of total gains
    // Depositor 2: 50% of total gains
    // Depositor 3: 25% of total gains (but only from second round)

    // Also verify asset gains maintain the same ratio
    let depositor_1_final_gain_round_2 = stability_pool_abi::get_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(depositor_1.address().into()),
        contracts.asset_contracts[0].asset_id,
    )
    .await
    .unwrap();

    let depositor_2_final_gain = stability_pool_abi::get_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(depositor_2.address().into()),
        contracts.asset_contracts[0].asset_id,
    )
    .await
    .unwrap();

    let depositor_3_final_gain = stability_pool_abi::get_depositor_asset_gain(
        &contracts.stability_pool,
        Identity::Address(depositor_3.address().into()),
        contracts.asset_contracts[0].asset_id,
    )
    .await
    .unwrap();

    assert_within_threshold(
        depositor_1_final_gain_round_2.value * 2,
        depositor_2_final_gain.value,
        "Asset gains should maintain 1:2 ratio between depositor 1 and 2",
    );

    assert_within_threshold(
        depositor_1_final_gain_round_2.value - depositor_1_asset_gain.value,
        depositor_3_final_gain.value,
        "Depositor 1 and 3 should have equal asset gains",
    );
}
