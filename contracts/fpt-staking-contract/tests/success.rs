use fuels::{prelude::*, types::Identity};
use test_utils::{
    interfaces::{
        borrow_operations::{borrow_operations_abi, BorrowOperations},
        fpt_staking::{fpt_staking_abi, FPTStaking},
        protocol_manager::{protocol_manager_abi, ProtocolManager},
        token::token_abi,
    },
    setup::common::setup_protocol,
};

#[tokio::test]
async fn proper_intialize() {
    let (contracts, admin, _wallets) = setup_protocol(10, 4, false).await;

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        5_000_000_000,
        Identity::Address(admin.address().into()),
    )
    .await;

    let pending_rewards_fpt = fpt_staking_abi::get_pending_usdf_gain(
        &contracts.fpt_staking,
        Identity::Address(admin.address().into()),
    )
    .await
    .value;
    assert_eq!(pending_rewards_fpt, 0);

    let pending_rewards_asset = fpt_staking_abi::get_pending_asset_gain(
        &contracts.fpt_staking,
        Identity::Address(admin.address().into()),
        contracts.asset_contracts[0].asset.contract_id().into(),
    )
    .await
    .value;
    assert_eq!(pending_rewards_asset, 0);
}

#[tokio::test]
async fn proper_staking_deposit() {
    let (contracts, admin, _wallets) = setup_protocol(10, 4, false).await;

    let provider = admin.provider().unwrap();

    let fpt_asset_id = AssetId::from(*contracts.fpt.contract_id().hash());

    token_abi::mint_to_id(
        &contracts.fpt,
        5_000_000_000,
        Identity::Address(admin.address().into()),
    )
    .await;

    fpt_staking_abi::stake(&contracts.fpt_staking, &contracts.fpt, 1_000_000_000).await;

    let fpt_balance = provider
        .get_asset_balance(admin.address().into(), fpt_asset_id)
        .await
        .unwrap();

    assert_eq!(fpt_balance, 4_000_000_000, "FPT Balance is wrong");
}

#[tokio::test]
async fn proper_staking_multiple_positions() {
    let (contracts, admin, mut wallets) = setup_protocol(10, 4, false).await;

    let provider = admin.provider().unwrap();

    let fpt_asset_id = AssetId::from(*contracts.fpt.contract_id().hash());
    let usdf_asset_id = AssetId::from(*contracts.usdf.contract_id().hash());
    let asset_id = AssetId::from(*contracts.asset_contracts[0].asset.contract_id().hash());

    let healthy_wallet1 = wallets.pop().unwrap();
    let healthy_wallet2 = wallets.pop().unwrap();
    let healthy_wallet3 = wallets.pop().unwrap();

    token_abi::mint_to_id(
        &contracts.fpt,
        5_000_000_000,
        Identity::Address(healthy_wallet1.address().into()),
    )
    .await;

    token_abi::mint_to_id(
        &contracts.fpt,
        5_000_000_000,
        Identity::Address(healthy_wallet2.address().into()),
    )
    .await;

    let fpt_staking_healthy_wallet1 = FPTStaking::new(
        contracts.fpt_staking.contract_id().clone(),
        healthy_wallet1.clone(),
    );

    let fpt_staking_healthy_wallet2 = FPTStaking::new(
        contracts.fpt_staking.contract_id().clone(),
        healthy_wallet2.clone(),
    );

    fpt_staking_abi::stake(&fpt_staking_healthy_wallet1, &contracts.fpt, 1_000_000_000).await;

    fpt_staking_abi::stake(&fpt_staking_healthy_wallet2, &contracts.fpt, 1_000_000_000).await;

    let fpt_balance_user1 = provider
        .get_asset_balance(healthy_wallet1.address().into(), fpt_asset_id)
        .await
        .unwrap();

    assert_eq!(fpt_balance_user1, 4_000_000_000, "FPT Balance is wrong");

    let fpt_balance_user1 = provider
        .get_asset_balance(healthy_wallet2.address().into(), fpt_asset_id)
        .await
        .unwrap();

    assert_eq!(fpt_balance_user1, 4_000_000_000, "FPT Balance is wrong");

    // basically we are going to open a trove, and through that generate some revenue for staking

    token_abi::mint_to_id(
        &contracts.asset_contracts[0].asset,
        40_000_000_000_000,
        Identity::Address(healthy_wallet3.address().into()),
    )
    .await;

    // let asset_user_balance = provider
    // .get_asset_balance(healthy_wallet3.address().into(), asset_id)
    // .await
    // .unwrap();

    // println!("Asset balance user {}", asset_user_balance);

    let borrow_operations_healthy_wallet3 = BorrowOperations::new(
        contracts.borrow_operations.contract_id().clone(),
        healthy_wallet3.clone(),
    );

    let _open_trove = borrow_operations_abi::open_trove(
        &borrow_operations_healthy_wallet3,
        &contracts.asset_contracts[0].oracle,
        &contracts.asset_contracts[0].asset,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.asset_contracts[0].sorted_troves,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.asset_contracts[0].active_pool,
        40_000_000_000_000,
        20_000_000_000_000,
        Identity::Address([0; 32].into()),
        Identity::Address([0; 32].into()),
    )
    .await
    .unwrap();

    let usdf_in_staking_balance = provider
        .get_contract_asset_balance(&contracts.fpt_staking.contract_id(), usdf_asset_id)
        .await
        .unwrap();

    // println!("USDF balance in staking contract {}", usdf_in_staking_balance);

    // let initial_usdf_user_balance = provider
    // .get_asset_balance(healthy_wallet3.address().into(), usdf_asset_id)
    // .await
    // .unwrap();

    // println!("USDF balance user {}", initial_usdf_user_balance);

    // let asset_user_balance = provider
    // .get_asset_balance(healthy_wallet3.address().into(), asset_id)
    // .await
    // .unwrap();

    // println!("Asset balance user {}", asset_user_balance);

    let redeem_amount = 10_000_000_000_000;

    let protocol_manager_healthy_wallet3 = ProtocolManager::new(
        contracts.protocol_manager.contract_id().clone(),
        healthy_wallet3.clone(),
    );
    protocol_manager_abi::redeem_collateral(
        &protocol_manager_healthy_wallet3,
        redeem_amount,
        10,
        0,
        0,
        None,
        None,
        &contracts.usdf,
        &contracts.fpt_staking,
        &contracts.asset_contracts,
    )
    .await;

    let asset_in_staking_balance = provider
        .get_contract_asset_balance(&contracts.fpt_staking.contract_id(), asset_id)
        .await
        .unwrap();

    // println!("ASSET balance in staking contract {}", asset_in_staking_balance);

    let _res1 = fpt_staking_abi::unstake(
        &fpt_staking_healthy_wallet1,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        &contracts.fpt,
        500_000_000,
    )
    .await;

    let _res2 = fpt_staking_abi::unstake(
        &fpt_staking_healthy_wallet2,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        &contracts.fpt,
        500_000_000,
    )
    .await;

    // println!("unstake");
    // print_response(&res);
    // println!("{:?}", &res.receipts);

    let fpt_balance_user1 = provider
        .get_asset_balance(healthy_wallet1.address().into(), fpt_asset_id)
        .await
        .unwrap();

    assert_eq!(fpt_balance_user1, 4_500_000_000, "FPT Balance is wrong");

    let fpt_balance_user2 = provider
        .get_asset_balance(healthy_wallet2.address().into(), fpt_asset_id)
        .await
        .unwrap();

    assert_eq!(fpt_balance_user2, 4_500_000_000, "FPT Balance is wrong");

    let usdf_user1_balance = provider
        .get_asset_balance(healthy_wallet1.address().into(), usdf_asset_id)
        .await
        .unwrap();

    // println!("USDF balance user {}", usdf_user1_balance);

    let usdf_user2_balance = provider
        .get_asset_balance(healthy_wallet2.address().into(), usdf_asset_id)
        .await
        .unwrap();

    // println!("USDF balance user {}", usdf_user2_balance);

    assert_eq!(
        usdf_user1_balance, usdf_user2_balance,
        "users usdf rewards don't match"
    );

    // println!("Should receive (together) usdf {}", usdf_in_staking_balance);

    assert_eq!(
        usdf_user1_balance + usdf_user2_balance,
        usdf_in_staking_balance,
        "Users did not receive exactly all the usdf staking rewards"
    );

    let asset_user1_balance = provider
        .get_asset_balance(healthy_wallet1.address().into(), asset_id)
        .await
        .unwrap();

    // println!("Asset balance user {}", asset_user1_balance);

    let asset_user2_balance = provider
        .get_asset_balance(healthy_wallet2.address().into(), asset_id)
        .await
        .unwrap();

    // println!("Asset balance user {}", asset_user2_balance);

    assert_eq!(
        asset_user1_balance, asset_user2_balance,
        "users asset rewards balance don't match"
    );

    // println!("Should receive (together) asset {}", asset_in_staking_balance);

    // let asset_in_staking_balance_after = provider
    // .get_contract_asset_balance(&contracts.fpt_staking.contract_id(), asset_id)
    // .await
    // .unwrap();

    // println!("ASSET balance in staking contract {}", asset_in_staking_balance_after);

    assert_eq!(
        asset_user1_balance + asset_user2_balance,
        asset_in_staking_balance,
        "Users did not receive exactly all the asset staking rewards"
    );
}