use fuels::prelude::abigen;
use fuels::programs::responses::CallResponse;

abigen!(Contract(
    name = "Oracle",
    abi = "contracts/mock-oracle-contract/out/debug/mock-oracle-contract-abi.json"
));

pub mod oracle_abi {

    use super::*;
    use fuels::prelude::{Account, TxPolicies};

    pub async fn set_price<T: Account>(oracle: &Oracle<T>, price: u64) -> CallResponse<()> {
        let tx_params = TxPolicies::default().with_tip(1);

        let res = oracle
            .methods()
            .set_price(price)
            .with_tx_policies(tx_params)
            .call()
            .await;

        return res.unwrap();
    }

    pub async fn get_price<T: Account>(oracle: &Oracle<T>) -> CallResponse<u64> {
        let tx_params = TxPolicies::default().with_tip(1);
        oracle
            .methods()
            .get_price()
            .with_tx_policies(tx_params)
            .call()
            .await
            .unwrap()
    }
}
