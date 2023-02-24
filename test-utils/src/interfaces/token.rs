use fuels::{
    prelude::{abigen, WalletUnlocked},
    programs::call_response::FuelCallResponse,
    types::Identity,
};

abigen!(
    Contract(
        name = "Token",
        abi = "contracts/token-contract/out/debug/token-contract-abi.json"
    ),
    Contract(
        name = "TroveManagerContract",
        abi = "contracts/trove-manager-contract/out/debug/trove-manager-contract-abi.json"
    )
);

pub async fn initialize(
    instance: &Token,
    amount: u64,
    admin: &WalletUnlocked,
    mut name: String,
    mut symbol: String,
) -> FuelCallResponse<()> {
    name.push_str(" ".repeat(32 - name.len()).as_str());
    symbol.push_str(" ".repeat(8 - symbol.len()).as_str());

    let config = TokenInitializeConfig {
        name: fuels::types::SizedAsciiString::<32>::new(name).unwrap(),
        symbol: fuels::types::SizedAsciiString::<8>::new(symbol).unwrap(),
        decimals: 6,
    };

    instance
        .methods()
        .initialize(config, amount, Identity::Address(admin.address().into()))
        .call()
        .await
        .unwrap()
}

pub async fn mint_to_id(
    instance: &Token,
    amount: u64,
    admin: &WalletUnlocked,
) -> FuelCallResponse<()> {
    instance
        .methods()
        .mint_to_id(amount, Identity::Address(admin.address().into()))
        .append_variable_outputs(1)
        .call()
        .await
        .unwrap()
}
