// References:
// https://docs.pyth.network/price-feeds/contract-addresses/fuel
// https://github.com/FuelLabs/verified-assets/blob/main/ASSETS.md

pub const TESTNET_CONTRACTS_FILE: &str = "testnet.contracts.json";
pub const MAINNET_CONTRACTS_FILE: &str = "mainnet.contracts.json";

pub const TESTNET_RPC: &str = "https://testnet.fuel.network/v1/playground";
pub const MAINNET_RPC: &str = "https://mainnet.fuel.network/v1/playground";

pub const TESTNET_TREASURY_IDENTITY: &str =
    "0xa5ac02c203dde9b52cb2ab29bdd0dfee1e7a17f97339ff2ead92de4eebb62305";
pub const MAINNET_TREASURY_IDENTITY: &str =
    "0x83953cdfeac61219ceb336684cc194d37d1fabfb8acbd530ba301ea241354280";

pub const TESTNET_PYTH_CONTRACT_ID: &str =
    "0x25146735b29d4216639f7f8b1d7b921ff87a1d3051de62d6cceaacabeb33b8e7";
pub const MAINNET_PYTH_CONTRACT_ID: &str =
    "0x1c86fdd9e0e7bc0d2ae1bf6817ef4834ffa7247655701ee1b031b52a24c523da";

// Testnet
pub const TESTNET_ETH_ASSET_CONTRACT_ID: &str =
    "0x4ea6ccef1215d9479f1024dff70fc055ca538215d2c8c348beddffd54583d0e8";
pub const TESTNET_ETH_ASSET_ID: &str =
    "f8f8b6283d7fa5b672b530cbb84fcccb4ff8dc40f8176ef4544ddb1f1952ad07";

// Mainnet
// https://github.com/FuelLabs/verified-assets/blob/main/ASSETS.md
pub const MAINNET_ASSET_CONTRACT_ID: &str =
    "0x4ea6ccef1215d9479f1024dff70fc055ca538215d2c8c348beddffd54583d0e8";
pub const MAINNET_ETH_ASSET_ID: &str =
    "0xf8f8b6283d7fa5b672b530cbb84fcccb4ff8dc40f8176ef4544ddb1f1952ad07";
pub const MAINNET_ETH_DECIMALS: u32 = 9;
pub const MAINNET_WSTETH_ASSET_ID: &str =
    "0x1a7815cc9f75db5c24a5b0814bfb706bb9fe485333e98254015de8f48f84c67b";
pub const MAINNET_WSTETH_DECIMALS: u32 = 9;
pub const MAINNET_EZETH_ASSET_ID: &str =
    "0x91b3559edb2619cde8ffb2aa7b3c3be97efd794ea46700db7092abeee62281b0";
pub const MAINNET_EZETH_DECIMALS: u32 = 9;
pub const MAINNET_PZETH_ASSET_ID: &str =
    "0x1493d4ec82124de8f9b625682de69dcccda79e882b89a55a8c737b12de67bd68";
pub const MAINNET_PZETH_DECIMALS: u32 = 9;
pub const MAINNET_WEETH_ASSET_ID: &str =
    "0x239ed6e12b7ce4089ee245244e3bf906999a6429c2a9a445a1e1faf56914a4ab";
pub const MAINNET_WEETH_DECIMALS: u32 = 9;
pub const MAINNET_RSETH_ASSET_ID: &str =
    "0xbae80f7fb8aa6b90d9b01ef726ec847cc4f59419c4d5f2ea88fec785d1b0e849";
pub const MAINNET_RSETH_DECIMALS: u32 = 9;
pub const MAINNET_METH_ASSET_ID: &str =
    "0xafd219f513317b1750783c6581f55530d6cf189a5863fd18bd1b3ffcec1714b4";
pub const MAINNET_METH_DECIMALS: u32 = 9;

// pyth price ids
// https://www.pyth.network/developers/price-feed-ids#pyth-evm-stable
pub const PYTH_ETH_PRICE_ID: &str =
    "ff61491a931112ddf1bd8147cd1b641375f79f5825126d665480874634fd0ace";
pub const PYTH_WSTETH_PRICE_ID: &str =
    "6df640f3b8963d8f8358f791f352b8364513f6ab1cca5ed3f1f7b5448980e784";
pub const PYTH_EZETH_PRICE_ID: &str =
    "06c217a791f5c4f988b36629af4cb88fad827b2485400a358f3b02886b54de92";
pub const PYTH_WEETH_PRICE_ID: &str =
    "9ee4e7c60b940440a261eb54b6d8149c23b580ed7da3139f7f08f4ea29dad395";
pub const PYTH_RSETH_PRICE_ID: &str =
    "0caec284d34d836ca325cf7b3256c078c597bc052fbd3c0283d52b581d68d71f";
pub const PYTH_METH_PRICE_ID: &str =
    "fbc9c3a716650b6e24ab22ab85b1c0ef4141b18f4590cc0b986e2f9064cf73d6";
pub const PYTH_PZETH_PRICE_ID: &str = ""; // Waiting for Pyth price feed

pub struct AssetConstants {
    pub symbol: &'static str,
    pub asset_contract_id: Option<&'static str>,
    pub asset_id: Option<&'static str>,
    pub pyth_contract_id: &'static str,
    pub pyth_price_id: &'static str,
    pub decimals: u32,
}

// Asset-specific constants
pub const TESTNET_ETH_CONSTANTS: AssetConstants = AssetConstants {
    symbol: "ETH",
    asset_contract_id: None,
    asset_id: None,
    pyth_contract_id: TESTNET_PYTH_CONTRACT_ID,
    pyth_price_id: PYTH_ETH_PRICE_ID,
    decimals: 9,
};

pub const TESTNET_WSTETH_CONSTANTS: AssetConstants = AssetConstants {
    symbol: "WSTETH",
    asset_contract_id: None,
    asset_id: None,
    pyth_contract_id: TESTNET_PYTH_CONTRACT_ID,
    pyth_price_id: PYTH_WSTETH_PRICE_ID,
    decimals: 9,
};

pub const TESTNET_EZETH_CONSTANTS: AssetConstants = AssetConstants {
    symbol: "EZETH",
    asset_contract_id: None,
    asset_id: None,
    pyth_contract_id: TESTNET_PYTH_CONTRACT_ID,
    pyth_price_id: PYTH_EZETH_PRICE_ID,
    decimals: 9,
};

pub const TESTNET_WEETH_CONSTANTS: AssetConstants = AssetConstants {
    symbol: "WEETH",
    asset_contract_id: None,
    asset_id: None,
    pyth_contract_id: TESTNET_PYTH_CONTRACT_ID,
    pyth_price_id: PYTH_WEETH_PRICE_ID,
    decimals: 9,
};

pub const TESTNET_RSETH_CONSTANTS: AssetConstants = AssetConstants {
    symbol: "RSETH",
    asset_contract_id: None,
    asset_id: None,
    pyth_contract_id: TESTNET_PYTH_CONTRACT_ID,
    pyth_price_id: PYTH_RSETH_PRICE_ID,
    decimals: 9,
};

pub const TESTNET_METH_CONSTANTS: AssetConstants = AssetConstants {
    symbol: "METH",
    asset_contract_id: None,
    asset_id: None,
    pyth_contract_id: TESTNET_PYTH_CONTRACT_ID,
    pyth_price_id: PYTH_METH_PRICE_ID,
    decimals: 9,
};

// Mainnet
pub const MAINNET_ETH_CONSTANTS: AssetConstants = AssetConstants {
    symbol: "ETH",
    asset_contract_id: Some(MAINNET_ASSET_CONTRACT_ID),
    asset_id: Some(MAINNET_ETH_ASSET_ID),
    pyth_contract_id: MAINNET_PYTH_CONTRACT_ID,
    pyth_price_id: PYTH_ETH_PRICE_ID,
    decimals: MAINNET_ETH_DECIMALS,
};

pub const MAINNET_WSTETH_CONSTANTS: AssetConstants = AssetConstants {
    symbol: "WSTETH",
    asset_contract_id: Some(MAINNET_ASSET_CONTRACT_ID),
    asset_id: Some(MAINNET_WSTETH_ASSET_ID),
    pyth_contract_id: MAINNET_PYTH_CONTRACT_ID,
    pyth_price_id: PYTH_WSTETH_PRICE_ID,
    decimals: MAINNET_WSTETH_DECIMALS,
};

pub const MAINNET_EZETH_CONSTANTS: AssetConstants = AssetConstants {
    symbol: "EZETH",
    asset_contract_id: Some(MAINNET_ASSET_CONTRACT_ID),
    asset_id: Some(MAINNET_EZETH_ASSET_ID),
    pyth_contract_id: MAINNET_PYTH_CONTRACT_ID,
    pyth_price_id: PYTH_EZETH_PRICE_ID,
    decimals: MAINNET_EZETH_DECIMALS,
};

pub const MAINNET_WEETH_CONSTANTS: AssetConstants = AssetConstants {
    symbol: "WEETH",
    asset_contract_id: Some(MAINNET_ASSET_CONTRACT_ID),
    asset_id: Some(MAINNET_WEETH_ASSET_ID),
    pyth_contract_id: MAINNET_PYTH_CONTRACT_ID,
    pyth_price_id: PYTH_WEETH_PRICE_ID,
    decimals: MAINNET_WEETH_DECIMALS,
};

pub const MAINNET_RSETH_CONSTANTS: AssetConstants = AssetConstants {
    symbol: "RSETH",
    asset_contract_id: Some(MAINNET_ASSET_CONTRACT_ID),
    asset_id: Some(MAINNET_RSETH_ASSET_ID),
    pyth_contract_id: MAINNET_PYTH_CONTRACT_ID,
    pyth_price_id: PYTH_RSETH_PRICE_ID,
    decimals: MAINNET_RSETH_DECIMALS,
};

pub const MAINNET_METH_CONSTANTS: AssetConstants = AssetConstants {
    symbol: "METH",
    asset_contract_id: Some(MAINNET_ASSET_CONTRACT_ID),
    asset_id: Some(MAINNET_METH_ASSET_ID),
    pyth_contract_id: MAINNET_PYTH_CONTRACT_ID,
    pyth_price_id: PYTH_METH_PRICE_ID,
    decimals: MAINNET_METH_DECIMALS,
};

// pub const MAINNET_PZETH_CONSTANTS: AssetConstants = AssetConstants {
//     symbol: "PZETH",
//     asset_contract_id: Some(MAINNET_ASSET_CONTRACT_ID),
//     asset_id: Some(MAINNET_PZETH_ASSET_ID),
//     pyth_contract_id: MAINNET_PYTH_CONTRACT_ID,
//     pyth_price_id: PYTH_PZETH_PRICE_ID,
//     decimals: MAINNET_PZETH_DECIMALS,
// };
