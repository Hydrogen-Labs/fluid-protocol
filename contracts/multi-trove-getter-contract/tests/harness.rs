use test_utils::interfaces::borrow_operations::borrow_operations_utils;
use test_utils::interfaces::oracle::oracle_abi;
use test_utils::interfaces::pyth_oracle::{pyth_oracle_abi, pyth_price_feed, PYTH_TIMESTAMP};
use test_utils::{
    data_structures::PRECISION,
    interfaces::multi_trove_getter::multi_trove_getter_abi,
    setup::common::{deploy_multi_trove_getter, setup_protocol},
};

#[tokio::test]
async fn test_get_multiple_sorted_troves() {
    let (contracts, _admin, mut wallets) = setup_protocol(20, false, false).await;
    let wallet = wallets.pop().unwrap();

    let multi_trove_getter = deploy_multi_trove_getter(
        &wallet,
        &contracts.sorted_troves.contract.contract_id().into(),
    )
    .await;

    // create 10 troves each with 600 USDF debt and n * 1000 collateral
    oracle_abi::set_debug_timestamp(&contracts.asset_contracts[0].oracle, PYTH_TIMESTAMP).await;
    pyth_oracle_abi::update_price_feeds(
        &contracts.asset_contracts[0].mock_pyth_oracle,
        pyth_price_feed(1),
    )
    .await;

    for i in 1..=10 {
        let wallet = wallets.pop().unwrap();
        let amount = i * 1000 * PRECISION;
        let usdf_amount = 600 * PRECISION;

        borrow_operations_utils::mint_token_and_open_trove(
            wallet.clone(),
            &contracts.asset_contracts[0],
            &contracts.borrow_operations,
            &contracts.usdf,
            &contracts.fpt_staking,
            &contracts.active_pool,
            &contracts.sorted_troves,
            amount,
            usdf_amount,
        )
        .await;
    }

    // Test getting multiple sorted troves
    let start_index = 0;
    let count = 10;

    let res = multi_trove_getter_abi::get_multiple_sorted_troves(
        &multi_trove_getter,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].asset_id,
        start_index,
        count,
    )
    .await
    .unwrap();

    let troves = res.value;
    assert_eq!(troves.len(), count as usize);

    // Verify that the troves are sorted in descending order of ICR
    for i in 1..troves.len() {
        assert!(
            troves[i - 1].collateral / troves[i - 1].debt <= troves[i].collateral / troves[i].debt
        );
    }
}
