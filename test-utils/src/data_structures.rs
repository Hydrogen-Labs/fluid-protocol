use super::interfaces::{
    active_pool::ActivePool, borrow_operations::BorrowOperations,
    coll_surplus_pool::CollSurplusPool, community_issuance::CommunityIssuance,
    default_pool::DefaultPool, fpt_staking::FPTStaking, fpt_token::FPTToken, oracle::Oracle,
    protocol_manager::ProtocolManager, pyth_oracle::PythCore, redstone_oracle::RedstoneCore,
    sorted_troves::SortedTroves, stability_pool::StabilityPool, token::Token,
    trove_manager::TroveManagerContract, usdf_token::USDFToken, vesting::VestingContract,
};
use fuels::{
    accounts::Account,
    types::{AssetId, Bits256, ContractId, U256},
};
pub const PRECISION: u64 = 1_000_000_000;
pub const POST_LIQUIDATION_COLLATERAL_RATIO: u64 = 1_500_000_000;

pub struct ContractInstance<C> {
    pub contract: C,
    pub implementation_id: ContractId,
}

impl<C> ContractInstance<C> {
    pub fn new(contract: C, implementation_id: ContractId) -> Self {
        Self {
            contract,
            implementation_id,
        }
    }
}

pub struct ProtocolContracts<T: Account> {
    pub borrow_operations: ContractInstance<BorrowOperations<T>>,
    pub usdf: USDFToken<T>,
    pub stability_pool: StabilityPool<T>,
    pub protocol_manager: ProtocolManager<T>,
    pub asset_contracts: Vec<AssetContracts<T>>, // TODO: Change to AssetContractsOptionalRedstone but it's a big refactor
    pub fpt_staking: FPTStaking<T>,
    pub coll_surplus_pool: CollSurplusPool<T>,
    pub sorted_troves: SortedTroves<T>,
    pub default_pool: DefaultPool<T>,
    pub active_pool: ActivePool<T>,
    pub fpt_token: FPTToken<T>,
    pub community_issuance: CommunityIssuance<T>,
    pub vesting_contract: ContractInstance<VestingContract<T>>,
    pub fpt_asset_id: AssetId,
    pub usdf_asset_id: AssetId,
}

pub struct AssetContracts<T: Account> {
    pub asset: Token<T>,
    pub oracle: Oracle<T>,
    pub mock_pyth_oracle: PythCore<T>,
    pub mock_redstone_oracle: RedstoneCore<T>,
    pub trove_manager: TroveManagerContract<T>,
    pub asset_id: AssetId,
    pub pyth_price_id: Bits256,
    pub redstone_price_id: U256,
    pub redstone_precision: u32,
    pub fuel_vm_decimals: u32,
}
pub struct AssetContractsOptionalRedstone<T: Account> {
    pub symbol: String,
    pub asset: Token<T>,
    pub oracle: Oracle<T>,
    pub mock_pyth_oracle: PythCore<T>,
    pub trove_manager: TroveManagerContract<T>,
    pub asset_id: AssetId,
    pub pyth_price_id: Bits256,
    pub fuel_vm_decimals: u32,
    pub redstone_config: Option<RedstoneConfig>,
}

pub struct ExistingAssetContracts {
    pub symbol: String,
    pub asset: Option<AssetConfig>,
    pub pyth_oracle: Option<PythConfig>,
    pub redstone_oracle: Option<RedstoneConfig>,
}

pub struct AssetConfig {
    pub asset: ContractId,
    pub asset_id: AssetId,
    pub fuel_vm_decimals: u32,
}

pub struct PythConfig {
    pub contract: ContractId,
    pub price_id: Bits256,
}

pub struct RedstoneConfig {
    pub contract: ContractId,
    pub price_id: U256,
    pub precision: u32,
}
