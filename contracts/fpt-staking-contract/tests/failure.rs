use fuels::{prelude::*, types::Identity};
use test_utils::{
    data_structures::PRECISION,
    interfaces::{fpt_staking::fpt_staking_abi, fpt_token::fpt_token_abi},
    setup::common::setup_protocol,
};

#[tokio::test]
async fn fails_unstake_wrong_amount() {
    let (contracts, admin, _wallets) = setup_protocol(10, 4, false).await;

    fpt_token_abi::mint_to_id(
        &contracts.fpt_token,
        5_000 * PRECISION,
        Identity::Address(admin.address().into()),
    )
    .await;

    fpt_staking_abi::stake(&contracts.fpt_staking, &contracts.fpt_token, 1 * PRECISION).await;

    fpt_staking_abi::unstake(
        &contracts.fpt_staking,
        &contracts.usdf,
        &contracts.asset_contracts[0].asset,
        &contracts.fpt_token,
        1_000 * PRECISION,
    )
    .await
    .expect_err("Unstake incorrect amount allowed");
}
