use crate::utils::utils::{is_testnet, load_core_contracts, load_hint_helper, setup_wallet};
use dotenv::dotenv;
use test_utils::{
    data_structures::PRECISION,
    interfaces::{
        hint_helper::hint_helper_abi,
        oracle::oracle_abi,
        pyth_oracle::{pyth_oracle_abi, pyth_price_feed, PYTH_TIMESTAMP},
        trove_manager::trove_manager_abi,
    },
};

pub async fn test_hint_helper() {
    dotenv().ok();

    let wallet = setup_wallet().await;
    let address = wallet.address();
    println!("🔑 Using wallet address: {}", address);

    let is_testnet = is_testnet(wallet.clone()).await;
    let contracts = load_core_contracts(wallet.clone(), is_testnet);
    let hint_helper = load_hint_helper(wallet.clone(), is_testnet);

    // Test hint helper
    let num_iterations = 20;
    let random_seed = 0;
    let target_cr = 340 * PRECISION / 100; // roughly 340% CR

    println!("Getting hint for CR: {}%", target_cr * 100 / PRECISION);

    let res = hint_helper_abi::get_approx_hint(
        &hint_helper,
        &contracts.asset_contracts[0].trove_manager,
        &contracts.sorted_troves,
        &contracts.asset_contracts[0].asset_id,
        target_cr,
        num_iterations,
        random_seed,
    )
    .await;

    println!("Hint result:");
    println!("Found address: {:?}", res.value.0.clone());
    println!("Difference from target: {}", res.value.1);
    println!("Final random seed: {}", res.value.2);

    println!("Test hint helper completed successfully");
    let posion = trove_manager_abi::get_nominal_icr(
        &contracts.asset_contracts[0].trove_manager,
        res.value.0,
    )
    .await;
    println!("Nominal ICR: {}", posion.value);
}
