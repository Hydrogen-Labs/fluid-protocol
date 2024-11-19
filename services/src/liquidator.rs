use dotenv::dotenv;
use fuels::prelude::*;
use std::{thread::sleep, time::Duration};
use test_utils::{
    data_structures::PRECISION,
    interfaces::{
        multi_trove_getter::multi_trove_getter_abi, oracle::oracle_abi,
        trove_manager::trove_manager_abi,
    },
};
use utils::{is_testnet, load_core_contracts, load_multi_trove_getter, setup_wallet};

// Reference to setup_wallet and load_core_contracts from utils.rs
use deploy_scripts::utils::*;

pub struct LiquidatorConfig {
    pub check_interval: Duration,
    pub max_troves_per_batch: usize,
}

const MIN_COLLATERAL_RATIO: u128 = (PRECISION as u128) * 135 / 100;

impl Default for LiquidatorConfig {
    fn default() -> Self {
        Self {
            check_interval: Duration::from_secs(60), // Check every minute
            max_troves_per_batch: 50,
        }
    }
}

pub struct Liquidator {
    wallet: WalletUnlocked,
    config: LiquidatorConfig,
}

impl Liquidator {
    pub async fn new(config: Option<LiquidatorConfig>) -> Self {
        dotenv().ok();
        let wallet = setup_wallet().await;

        Self {
            wallet,
            config: config.unwrap_or_default(),
        }
    }

    pub async fn start(&self) -> Result<()> {
        println!("🔄 Starting liquidator service...");
        println!("🔑 Using wallet address: {}", self.wallet.address());

        let is_testnet = is_testnet(self.wallet.clone()).await;
        let core_contracts = load_core_contracts(self.wallet.clone(), is_testnet);
        let multi_trove_getter = load_multi_trove_getter(self.wallet.clone(), is_testnet);

        loop {
            println!("📊 Checking for liquidatable troves...");

            for asset_contract in core_contracts.asset_contracts.iter() {
                println!("🔍 Checking asset ID: 0x{}", asset_contract.asset_id);
                println!("   Symbol: {}", asset_contract.symbol);

                let price = match oracle_abi::get_price(
                    &asset_contract.oracle,
                    &asset_contract.mock_pyth_oracle,
                    &None,
                )
                .await
                {
                    Ok(response) => {
                        println!(
                            "📈 Current price: {:.9}",
                            response.value as f64 / PRECISION as f64
                        );
                        response.value
                    }
                    Err(e) => {
                        println!("❌ Error getting price from oracle: {:?}", e);
                        continue;
                    }
                };

                let mut liquidatable_troves = Vec::new();
                let mut start_idx = 0;
                loop {
                    println!("📑 Fetching troves batch starting at index: {}", start_idx);
                    let troves = match multi_trove_getter_abi::get_multiple_sorted_troves(
                        &multi_trove_getter,
                        &asset_contract.trove_manager,
                        &core_contracts.sorted_troves,
                        &asset_contract.asset_id,
                        start_idx,
                        10,
                    )
                    .await
                    {
                        Ok(response) => response.value,
                        Err(e) => {
                            println!("❌ Error getting troves batch: {:?}", e);
                            break;
                        }
                    };

                    if troves.is_empty() {
                        println!("✅ No more troves to check");
                        break;
                    }

                    println!("📝 Processing {} troves in current batch", troves.len());
                    let troves_len = troves.len();
                    for (i, trove) in troves.into_iter().enumerate() {
                        let cr = (trove.collateral as u128 * price as u128 * PRECISION as u128)
                            / (trove.debt as u128 * PRECISION as u128);
                        if i == 0 || cr < MIN_COLLATERAL_RATIO {
                            println!(
                                "   Trove CR: {:.9}, Min Required: {:.9}{}",
                                cr as f64 / PRECISION as f64,
                                MIN_COLLATERAL_RATIO as f64 / PRECISION as f64,
                                if i == 0 {
                                    " (Lowest CR)"
                                } else {
                                    " (Liquidatable)"
                                }
                            );
                        }
                        if cr < MIN_COLLATERAL_RATIO {
                            liquidatable_troves.push(trove.address);
                        }
                    }

                    start_idx += troves_len as u64;
                }

                if !liquidatable_troves.is_empty() {
                    println!(
                        "🚨 Found {} liquidatable troves for asset {}",
                        liquidatable_troves.len(),
                        asset_contract.asset_id
                    );

                    for (i, chunk) in liquidatable_troves
                        .chunks(self.config.max_troves_per_batch)
                        .enumerate()
                    {
                        println!("⚡ Processing batch {} with {} troves", i + 1, chunk.len());
                        match trove_manager_abi::batch_liquidate_troves(
                            &asset_contract.trove_manager,
                            &core_contracts.community_issuance,
                            &core_contracts.stability_pool,
                            &asset_contract.oracle,
                            &asset_contract.mock_pyth_oracle,
                            &asset_contract.mock_redstone_oracle,
                            &core_contracts.sorted_troves,
                            &core_contracts.active_pool,
                            &core_contracts.default_pool,
                            &core_contracts.coll_surplus_pool,
                            &core_contracts.usdf,
                            chunk.to_vec(),
                            fuels::types::Identity::Address(Address::zeroed()),
                            fuels::types::Identity::Address(Address::zeroed()),
                        )
                        .await
                        {
                            Ok(_) => println!(
                                "✅ Successfully liquidated batch {} of {} troves",
                                i + 1,
                                chunk.len()
                            ),
                            Err(e) => println!("❌ Failed to liquidate batch {}: {:?}", i + 1, e),
                        }
                    }
                } else {
                    println!(
                        "✅ No liquidatable troves found for asset {}",
                        asset_contract.symbol
                    );
                }
            }

            println!(
                "💤 Sleeping for {} seconds...\n",
                self.config.check_interval.as_secs()
            );
            sleep(self.config.check_interval);
        }
    }
}
