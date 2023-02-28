use fuels::{prelude::*, types::Identity};

use test_utils::{
    interfaces::active_pool::{active_pool_abi, ActivePool},
    interfaces::token::{token_abi, Token},
    setup::common::{deploy_active_pool, deploy_token},
};

async fn get_contract_instance() -> (ActivePool, Token, WalletUnlocked) {
    // Launch a local network and deploy the contract
    let mut wallets = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(
            Some(2),             /* Single wallet */
            Some(1),             /* Single coin (UTXO) */
            Some(1_000_000_000), /* Amount per coin */
        ),
        None,
        None,
    )
    .await;
    let wallet = wallets.pop().unwrap();

    let instance = deploy_active_pool(&wallet).await;

    let asset = deploy_token(&wallet).await;

    token_abi::initialize(
        &asset,
        1_000_000_000,
        &Identity::Address(wallet.address().into()),
        "Fuel".to_string(),
        "FUEL".to_string(),
    )
    .await;

    active_pool_abi::initialize(
        &instance,
        Identity::Address(wallet.address().into()),
        Identity::Address(wallet.address().into()),
        Identity::Address(wallet.address().into()),
        asset.contract_id().into(),
    )
    .await;

    (instance, asset, wallet)
}

#[tokio::test]
async fn proper_intialize() {
    let (active_pool, _mock_fuel, _admin) = get_contract_instance().await;

    let debt = active_pool_abi::get_usdf_debt(&active_pool).await.value;
    assert_eq!(debt, 0);

    let asset_amount = active_pool_abi::get_asset(&active_pool).await.value;
    assert_eq!(asset_amount, 0);
}

#[tokio::test]
async fn proper_adjust_debt() {
    let (active_pool, _mock_fuel, _admin) = get_contract_instance().await;

    active_pool_abi::increase_usdf_debt(&active_pool, 1000).await;

    let debt = active_pool_abi::get_usdf_debt(&active_pool).await.value;
    assert_eq!(debt, 1000);

    active_pool_abi::decrease_usdf_debt(&active_pool, 500).await;

    let debt = active_pool_abi::get_usdf_debt(&active_pool).await.value;
    assert_eq!(debt, 500);
}

#[tokio::test]
async fn proper_adjust_asset_col() {
    let (active_pool, mock_fuel, admin) = get_contract_instance().await;

    token_abi::mint_to_id(&mock_fuel, 1_000_000, &admin.clone()).await;

    active_pool_abi::recieve(&active_pool, &mock_fuel, 1_000_000).await;

    let asset_amount = active_pool_abi::get_asset(&active_pool).await.value;
    assert_eq!(asset_amount, 1_000_000);
}
