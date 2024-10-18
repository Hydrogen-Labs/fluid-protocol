use crate::data_structures::PRECISION;
use crate::interfaces::active_pool::ActivePool;
use crate::interfaces::borrow_operations::{borrow_operations_abi, BorrowOperations};
use crate::interfaces::fpt_staking::FPTStaking;
use crate::interfaces::oracle::{oracle_abi, Oracle};
use crate::interfaces::pyth_oracle::PythCore;
use crate::interfaces::redstone_oracle::RedstoneCore;
use crate::interfaces::sorted_troves::SortedTroves;
use crate::interfaces::token::Token;
use crate::interfaces::trove_manager::TroveManagerContract;
use crate::interfaces::usdf_token::USDFToken;
use dotenv::dotenv;
use fuels::prelude::{Bech32ContractId, Provider, WalletUnlocked};
use fuels::types::{Address, Identity};

const RPC: &str = "beta-4.fuel.network";

// This is not a core part of the testing suite, meant to be a quick script for manking manual queries to the testnet

#[tokio::main]
pub async fn testing_query() {
    let provider = match Provider::connect(RPC).await {
        Ok(p) => p,
        Err(error) => panic!("❌ Problem creating provider: {:#?}", error),
    };

    dotenv().ok();
    let secret = match std::env::var("SECRET") {
        Ok(s) => s,
        Err(error) => panic!("❌ Cannot find .env file: {:#?}", error),
    };

    let wallet = WalletUnlocked::new_from_mnemonic_phrase_with_path(
        &secret,
        Some(provider.clone()),
        "m/44'/1179993420'/0'/0/0",
    )
    .unwrap();

    println!("Wallet address: {}", wallet.address());
    let id: Bech32ContractId = "fuel129gw5u3rlacka3smhngevvgq4awllx8u4l5fktpr506yaxv8gx4qz6y4k3"
        .parse()
        .expect("Invalid ID");

    let oracle = Oracle::new(id.clone(), wallet.clone());
    let pyth = PythCore::new(id.clone(), wallet.clone());
    let redstone = Some(RedstoneCore::new(id, wallet.clone()));

    let res = oracle_abi::get_price(&oracle, &pyth, &redstone).await;

    println!("Result: {:#?}", res.value);

    let borrow_operations_id: Bech32ContractId =
        "fuel1wnys85mec9vna4y577r97w0u4egdpmnvuxv32cph8uqqzmx8694sd7wqtw"
            .parse()
            .expect("Invalid ID");

    let borrow_operations = BorrowOperations::new(borrow_operations_id, wallet.clone());

    let null_hint = Identity::Address(Address::default());

    let asset_token_id: Bech32ContractId =
        "fuel1ql6d5vjmuqs0v2tev7su73zjrpajffy9cjccvll38mxmamaeteuqml4pxl"
            .parse()
            .expect("Invalid ID");
    let asset_token = Token::new(asset_token_id, wallet.clone());

    let usdf_token_id: Bech32ContractId =
        "fuel1an59xymuwqj9r757agfcu0wetqadsl0lc6xw7xe3vka23d0z2tfqa8t7c5"
            .parse()
            .expect("Invalid ID");

    let usdf_token = USDFToken::new(usdf_token_id, wallet.clone());

    let fpt_staking_id: Bech32ContractId =
        "fuel14a5zgt9yz04rwnt7z7dyxuhtdlzyjtu9nfxw7pl3ares0zd85svqwlntrm"
            .parse()
            .expect("Invalid ID");

    let fpt_staking = FPTStaking::new(fpt_staking_id, wallet.clone());

    let sorted_troves_id: Bech32ContractId =
        "fuel17q7999tp3s55jk7ev9sj6kmzp3qfmr8rnwnf6dzg9df4z3jrh74qpg5x22"
            .parse()
            .expect("Invalid ID");

    let sorted_troves = SortedTroves::new(sorted_troves_id, wallet.clone());

    let trove_manager_id: Bech32ContractId =
        "fuel17thhl04jewnftymwksgufgcsegea72a6pjfwgxg0nptvc0ys5yjq05arr4"
            .parse()
            .expect("Invalid ID");

    let trove_manager = TroveManagerContract::new(trove_manager_id, wallet.clone());

    let active_pool_id: Bech32ContractId =
        "fuel12qxy3gk3wdm3cytlfsaegzth7cnn5de5q8hrg6cdukff2k0zhcws3rxqef"
            .parse()
            .expect("Invalid ID");

    let active_pool = ActivePool::new(active_pool_id, wallet.clone());

    let mock_amount_deposit = 2 * PRECISION;
    let usdf_amount_withdrawn = 600 * PRECISION;

    let _res = borrow_operations_abi::open_trove(
        &borrow_operations,
        &oracle,
        &pyth,
        &redstone,
        &asset_token,
        &usdf_token,
        &fpt_staking,
        &sorted_troves,
        &trove_manager,
        &active_pool,
        mock_amount_deposit,
        usdf_amount_withdrawn,
        null_hint.clone(),
        null_hint.clone(),
    )
    .await
    .unwrap();
}
