use crate::interfaces::{pyth_oracle::PythCore, redstone_oracle::RedstoneCore};
use fuels::prelude::abigen;
use fuels::programs::responses::CallResponse;

abigen!(Contract(
    name = "Oracle",
    abi = "contracts/oracle-contract/out/debug/oracle-contract-abi.json"
));

// 4 hours
pub const ORACLE_TIMEOUT: u64 = 14400;

pub mod oracle_abi {

    use super::*;
    use fuels::{
        prelude::{Account, TxPolicies},
        programs::calls::ContractDependency,
    };

    pub async fn get_price<T: Account>(
        oracle: &Oracle<T>,
        pyth: &PythCore<T>,
        redstone: &Option<RedstoneCore<T>>,
    ) -> CallResponse<u64> {
        let tx_params = TxPolicies::default().with_tip(1);
        let mut contracts: Vec<&dyn ContractDependency> = vec![pyth];

        if let Some(redstone) = redstone {
            contracts.push(redstone);
        }

        oracle
            .methods()
            .get_price()
            .with_contracts(&contracts)
            .with_tx_policies(tx_params)
            .call()
            .await
            .unwrap()
    }

    pub async fn set_debug_timestamp<T: Account>(oracle: &Oracle<T>, timestamp: u64) {
        let tx_params = TxPolicies::default().with_tip(1);

        oracle
            .methods()
            .set_debug_timestamp(timestamp)
            .with_tx_policies(tx_params)
            .call()
            .await
            .unwrap();
    }
}
