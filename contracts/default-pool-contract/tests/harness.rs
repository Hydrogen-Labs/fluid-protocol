use fuels::{prelude::*, types::Identity};

use test_utils::{
    data_structures::ContractInstance,
    interfaces::usdf_token::{usdf_token_abi, USDFToken},
    setup::common::deploy_usdf_token,
};

async fn get_contract_instance() -> (
    ContractInstance<USDFToken<WalletUnlocked>>,
    WalletUnlocked,
    Vec<WalletUnlocked>,
) {
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
    .await
    .unwrap();
    let wallet = wallets.pop().unwrap();

    let asset = deploy_usdf_token(&wallet).await;

    usdf_token_abi::initialize(
        &asset,
        asset.contract.contract_id().into(),
        Identity::Address(wallet.address().into()),
        Identity::Address(wallet.address().into()),
    )
    .await
    .unwrap();

    (asset, wallet, wallets)
}

#[tokio::test]
async fn proper_intialize() {
    let (usdf, _admin, _) = get_contract_instance().await;

    let total_supply = usdf_token_abi::total_supply(&usdf).await.value.unwrap();

    assert_eq!(total_supply, 0);
}

#[tokio::test]
async fn proper_mint() {
    let (usdf, _, mut wallets) = get_contract_instance().await;

    let wallet = wallets.pop().unwrap();

    usdf_token_abi::mint(&usdf, 100, Identity::Address(wallet.address().into()))
        .await
        .unwrap();

    let total_supply = usdf_token_abi::total_supply(&usdf).await.value.unwrap();

    assert_eq!(total_supply, 100);
}

#[tokio::test]
async fn proper_burn() {
    let (usdf, admin, _wallets) = get_contract_instance().await;

    usdf_token_abi::mint(&usdf, 100, Identity::Address(admin.address().into()))
        .await
        .unwrap();

    let total_supply = usdf_token_abi::total_supply(&usdf).await.value.unwrap();

    assert_eq!(total_supply, 100);

    usdf_token_abi::burn(&usdf, 50).await.unwrap();

    let total_supply = usdf_token_abi::total_supply(&usdf).await.value.unwrap();

    assert_eq!(total_supply, 50);
}

#[tokio::test]
async fn fails_to_mint_unauthorized() {
    let (usdf, _, mut wallets) = get_contract_instance().await;

    let wallet = wallets.pop().unwrap();

    let unauthorized_usdf = ContractInstance::new(
        USDFToken::new(usdf.contract.contract_id(), wallet.clone()),
        usdf.implementation_id.clone(),
    );

    usdf_token_abi::mint(
        &unauthorized_usdf,
        100,
        Identity::Address(wallet.address().into()),
    )
    .await
    .expect_err("Should fail to mint");
}

#[tokio::test]
async fn fails_to_burn_unauthorized() {
    let (usdf, _, mut wallets) = get_contract_instance().await;

    let wallet = wallets.pop().unwrap();

    usdf_token_abi::mint(&usdf, 100, Identity::Address(wallet.address().into()))
        .await
        .unwrap();

    let unauthorized_usdf = ContractInstance::new(
        USDFToken::new(usdf.contract.contract_id(), wallet.clone()),
        usdf.implementation_id.clone(),
    );

    usdf_token_abi::burn(&unauthorized_usdf, 100)
        .await
        .expect_err("Should fail to burn");
}
