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

pub struct ProtocolContracts<T: Account> {
    pub borrow_operations: BorrowOperations<T>,
    pub usdf: USDFToken<T>,
    pub stability_pool: StabilityPool<T>,
    pub protocol_manager: ProtocolManager<T>,
    pub asset_contracts: Vec<AssetContracts<T>>,
    pub fpt_staking: FPTStaking<T>,
    pub coll_surplus_pool: CollSurplusPool<T>,
    pub sorted_troves: SortedTroves<T>,
    pub default_pool: DefaultPool<T>,
    pub active_pool: ActivePool<T>,
    pub fpt_token: FPTToken<T>,
    pub community_issuance: CommunityIssuance<T>,
    pub vesting_contract: VestingContract<T>,
    pub fpt_asset_id: AssetId,
    pub usdf_asset_id: AssetId,
}

pub struct AssetContracts<T: Account> {
    pub asset: Token<T>,
    pub oracle: Oracle<T>,
    pub mock_pyth_oracle: PythCore<T>,
    pub mock_redstone_oracle: Option<RedstoneCore<T>>,
    pub trove_manager: TroveManagerContract<T>,
    pub asset_id: AssetId,
    pub pyth_price_id: Bits256,
    pub pyth_precision: u8,
    pub redstone_price_id: U256,
    pub redstone_precision: u8,
}

pub struct ExistingAssetContracts {
    pub asset: ContractId,
    pub asset_id: AssetId,
    pub pyth_oracle: ContractId,
    pub pyth_price_id: Bits256,
    pub pyth_precision: u8,
    pub redstone_oracle: Option<ContractId>,
    pub redstone_price_id: U256,
    pub redstone_precision: u8,
}
