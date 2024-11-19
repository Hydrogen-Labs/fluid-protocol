use fuels::{prelude::*, types::Identity};
use test_utils::{
    data_structures::ContractInstance,
    interfaces::{
        oracle::{oracle_abi, Oracle, RedstoneConfig},
        pyth_oracle::{pyth_oracle_abi, Price, PythCore, DEFAULT_PYTH_PRICE_ID},
    },
    setup::common::{deploy_mock_pyth_oracle, deploy_mock_redstone_oracle, deploy_oracle},
};

async fn setup() -> (
    ContractInstance<Oracle<WalletUnlocked>>,
    PythCore<WalletUnlocked>,
    WalletUnlocked,
    WalletUnlocked,
) {
    let mut wallets = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(Some(3), Some(1), Some(1_000_000_000)),
        None,
        None,
    )
    .await
    .unwrap();

    let deployer_wallet = wallets.pop().unwrap();
    let attacker_wallet = wallets.pop().unwrap();

    let pyth = deploy_mock_pyth_oracle(&deployer_wallet).await;

    let oracle = deploy_oracle(
        &deployer_wallet,
        pyth.contract_id().into(),
        DEFAULT_PYTH_PRICE_ID,
        9, // Default Fuel VM decimals
        true,
        Identity::Address(deployer_wallet.address().into()),
    )
    .await;

    (oracle, pyth, deployer_wallet, attacker_wallet)
}

#[tokio::test]
async fn test_set_redstone_config_authorization() {
    let (oracle, _, deployer_wallet, attacker_wallet) = setup().await;
    let redstone = deploy_mock_redstone_oracle(&deployer_wallet).await;
    // Test 1: Authorized set_redstone_config
    let redstone_config = RedstoneConfig {
        contract_id: ContractId::from([1u8; 32]),
        price_id: [2u8; 32].into(),
        precision: 6,
    };

    let result = oracle_abi::set_redstone_config(&oracle, &redstone, redstone_config.clone()).await;
    assert!(
        result.is_ok(),
        "Authorized user should be able to set Redstone config"
    );

    // Test 2: Unauthorized set_redstone_config
    let oracle_attacker = ContractInstance::new(
        Oracle::new(
            oracle.contract.contract_id().clone(),
            attacker_wallet.clone(),
        ),
        oracle.implementation_id,
    );

    let result =
        oracle_abi::set_redstone_config(&oracle_attacker, &redstone, redstone_config.clone()).await;

    assert!(
        result.is_err(),
        "Unauthorized user should not be able to set Redstone config"
    );
    if let Err(error) = result {
        assert!(
            error
                .to_string()
                .contains("Only initializer can set Redstone config"),
            "Unexpected error message: {}",
            error
        );
    }

    // Test 3: Attempt to set Redstone config again (should fail)
    let result = oracle_abi::set_redstone_config(&oracle, &redstone, redstone_config.clone()).await;
    assert!(result.is_err(), "Setting Redstone config twice should fail");
    if let Err(error) = result {
        assert!(
            error.to_string().contains("Redstone config already set"),
            "Unexpected error message: {}",
            error
        );
    }
}

#[tokio::test]
async fn test_get_price_pyth_only() {
    let (oracle, pyth, _, _) = setup().await;

    // Set a price in Pyth
    let pyth_price = 1000 * 1_000_000_000; // $1000 with 9 decimal places
    let pyth_timestamp = 1234567890;

    oracle_abi::set_debug_timestamp(&oracle, pyth_timestamp).await;
    pyth_oracle_abi::update_price_feeds(
        &pyth,
        vec![(
            DEFAULT_PYTH_PRICE_ID,
            Price {
                confidence: 0,
                exponent: 9,
                price: pyth_price,
                publish_time: pyth_timestamp,
            },
        )],
    )
    .await;

    // Get price from Oracle (should return Pyth price)
    let price = oracle_abi::get_price(&oracle, &pyth, &None)
        .await
        .unwrap()
        .value;
    assert_eq!(price, pyth_price, "Oracle should return Pyth price");

    // Set Pyth price as stale
    let stale_timestamp = pyth_timestamp + 14401; // TIMEOUT + 1
    oracle_abi::set_debug_timestamp(&oracle, stale_timestamp).await;

    // Get price from Oracle (should return last good price)
    let price = oracle_abi::get_price(&oracle, &pyth, &None)
        .await
        .unwrap()
        .value;
    assert_eq!(
        price, pyth_price,
        "Oracle should return last good price when Pyth is stale"
    );
}
