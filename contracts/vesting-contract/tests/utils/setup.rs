use fuels::prelude::*;
use test_utils::{
    interface::{abigen_bindings::token_mod::TokenInitializeConfig, Token},
    setup::common::deploy_token,
};
// TODO: do setup instead of copy/pasted code with minor adjustments

// Load abi from json
abigen!(Contract(
    name = "VestingContract",
    abi = "contracts/vesting-contract/out/debug/vesting-contract-abi.json"
));

const VESTING_CONTRACT_BINARY_PATH: &str = "out/debug/vesting-contract.bin";
const VESTING_CONTRACT_STORAGE_PATH: &str = "out/debug/vesting-contract-storage_slots.json";

pub async fn setup() -> (VestingContract, WalletUnlocked, WalletUnlocked, Token) {
    let config = Config {
        manual_blocks_enabled: true, // Necessary so the `produce_blocks` API can be used locally
        ..Config::local_node()
    };

    let mut wallets = launch_custom_provider_and_get_wallets(
        WalletsConfig::new(
            Some(2),             /* Single wallet */
            Some(1),             /* Single coin (UTXO) */
            Some(1_000_000_000), /* Amount per coin */
        ),
        Some(config),
        None,
    )
    .await;

    let wallet = wallets.pop().unwrap();
    let wallet2 = wallets.pop().unwrap();

    let id = Contract::deploy(
        VESTING_CONTRACT_BINARY_PATH,
        &wallet,
        TxParameters::default(),
        StorageConfiguration::with_storage_path(Some(VESTING_CONTRACT_STORAGE_PATH.to_string())),
    )
    .await
    .unwrap();

    let instance = VestingContract::new(id.clone(), wallet.clone());

    let asset = deploy_token(&wallet).await;

    (instance, wallet, wallet2, asset)
}

pub mod test_helpers {
    use fuels::programs::call_response::FuelCallResponse;
    use fuels::types::Identity;

    use super::abigen_bindings::vesting_contract_mod::{Asset, VestingSchedule};
    use super::*;

    pub async fn mint_to_vesting(
        contract: &Token,
        vesting_contract: &VestingContract,
        amount: u64,
        admin: &WalletUnlocked,
    ) {
        let instance = Token::new(contract.id().clone(), admin.clone());
        let asset_id = AssetId::from(*instance.id().hash());
        let mut name = "Fluid Protocol Test Token".to_string();
        let mut symbol = "FPTT".to_string();

        name.push_str(" ".repeat(32 - name.len()).as_str());
        symbol.push_str(" ".repeat(8 - symbol.len()).as_str());

        let config = TokenInitializeConfig {
            name: fuels::types::SizedAsciiString::<32>::new(name).unwrap(),
            symbol: fuels::types::SizedAsciiString::<8>::new(symbol).unwrap(),
            decimals: 6,
        };

        let _ = instance
            .methods()
            .initialize(config, amount, Identity::Address(admin.address().into()))
            .call()
            .await;

        let res = instance
            .methods()
            .mint_to_id(amount, Identity::Address(admin.address().into()))
            .append_variable_outputs(1)
            .call()
            .await;

        println!("res: {:?}", res);

        let _res = admin
            .force_transfer_to_contract(
                &vesting_contract.id(),
                amount,
                asset_id,
                TxParameters::default(),
            )
            .await;
    }

    pub async fn instantiate_vesting_contract(
        contract: &VestingContract,
        admin: &Address,
        vesting_schedule: &Vec<VestingSchedule>,
        asset_contract: &Token,
        amount: u64,
    ) -> Result<FuelCallResponse<()>> {
        let asset: Asset = Asset {
            id: asset_contract.id().into(),
            amount,
        };

        contract
            .methods()
            .constructor(
                Identity::Address(admin.clone()),
                vesting_schedule.clone(),
                asset.clone(),
            )
            .call()
            .await
    }

    pub fn get_vesting_schedule(
        cliff_amount: u64,
        cliff_timestamp: u64,
        end_timestamp: u64,
        claimed_amount: u64,
        total_amount: u64,
        recipient: Identity,
        revocable: bool,
    ) -> VestingSchedule {
        VestingSchedule {
            cliff_amount,
            cliff_timestamp,
            end_timestamp,
            claimed_amount,
            total_amount,
            recipient,
            revocable,
        }
    }
}
