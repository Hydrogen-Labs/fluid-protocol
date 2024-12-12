use crate::data_structures::PRECISION;
use fuels::prelude::abigen;
use fuels::programs::responses::CallResponse;
use fuels::types::Bits256;

abigen!(Contract(
    name = "PythCore",
    abi = "contracts/mock-pyth-contract/out/debug/mock-pyth-contract-abi.json"
));

pub const DEFAULT_PYTH_PRICE_ID: Bits256 = Bits256([0; 32]);
pub const TAI64_UNIX_ADJUSTMENT: u64 = 10 + (1 << 62);
pub const PYTH_TIMESTAMP: u64 = 1724166967 + TAI64_UNIX_ADJUSTMENT;
pub const PYTH_PRECISION: u32 = 9;

pub fn pyth_price_feed(price: u64) -> Vec<(Bits256, Price)> {
    vec![(
        Bits256::zeroed(),
        Price {
            confidence: 0,
            exponent: 9,
            price: price * PRECISION,
            publish_time: PYTH_TIMESTAMP,
        },
    )]
}

pub fn pyth_price_feed_with_time(
    price: u64,
    unix_timestamp: u64,
    exponent: u32,
) -> Vec<(Bits256, Price)> {
    vec![(
        Bits256::zeroed(),
        Price {
            confidence: 0,
            exponent,
            price: price * PRECISION,
            publish_time: unix_timestamp,
        },
    )]
}

pub fn pyth_price_feed_with_confidence(
    price: u64,
    unix_timestamp: u64,
    confidence: u64,
    exponent: u32,
) -> Vec<(Bits256, Price)> {
    vec![(
        Bits256::zeroed(),
        Price {
            confidence: confidence,
            exponent,
            price: price,
            publish_time: unix_timestamp,
        },
    )]
}

pub fn pyth_price_no_precision_with_time(price: u64, unix_timestamp: u64) -> Vec<(Bits256, Price)> {
    vec![(
        Bits256::zeroed(),
        Price {
            confidence: 0,
            exponent: 9,
            price,
            publish_time: unix_timestamp,
        },
    )]
}

pub mod pyth_oracle_abi {

    use super::*;
    use fuels::prelude::{Account, TxPolicies};

    pub async fn price<T: Account>(
        oracle: &PythCore<T>,
        price_feed_id: &Bits256,
    ) -> CallResponse<Price> {
        let tx_params = TxPolicies::default().with_tip(1);
        oracle
            .methods()
            .price(price_feed_id.clone())
            .with_tx_policies(tx_params)
            .call()
            .await
            .unwrap()
    }

    pub async fn price_unsafe<T: Account>(
        oracle: &PythCore<T>,
        price_feed_id: &Bits256,
    ) -> CallResponse<Price> {
        let tx_params = TxPolicies::default().with_tip(1);
        oracle
            .methods()
            .price_unsafe(price_feed_id.clone())
            .with_tx_policies(tx_params)
            .call()
            .await
            .unwrap()
    }

    pub async fn update_price_feeds<T: Account>(
        oracle: &PythCore<T>,
        feeds: Vec<(Bits256, Price)>,
    ) {
        let tx_params = TxPolicies::default().with_tip(1);

        oracle
            .methods()
            .update_price_feeds(feeds)
            .with_tx_policies(tx_params)
            .call()
            .await
            .unwrap();
    }
}
