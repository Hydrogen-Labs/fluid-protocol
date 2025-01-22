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

impl<C: Clone> ContractInstance<C> {
    pub fn new(contract: C, implementation_id: ContractId) -> Self {
        Self {
            contract,
            implementation_id,
        }
    }
}

impl<C: Clone> Clone for ContractInstance<C> {
    fn clone(&self) -> Self {
        Self {
            contract: self.contract.clone(),
            implementation_id: self.implementation_id.clone(),
        }
    }
}

pub struct ProtocolContracts<T: Account> {
    pub borrow_operations: ContractInstance<BorrowOperations<T>>,
    pub usdf: ContractInstance<USDFToken<T>>,
    pub stability_pool: ContractInstance<StabilityPool<T>>,
    pub protocol_manager: ContractInstance<ProtocolManager<T>>,
    pub asset_contracts: Vec<AssetContracts<T>>, // TODO: Change to AssetContractsOptionalRedstone but it's a big refactor
    pub fpt_staking: ContractInstance<FPTStaking<T>>,
    pub coll_surplus_pool: ContractInstance<CollSurplusPool<T>>,
    pub sorted_troves: ContractInstance<SortedTroves<T>>,
    pub default_pool: ContractInstance<DefaultPool<T>>,
    pub active_pool: ContractInstance<ActivePool<T>>,
    pub fpt_token: ContractInstance<FPTToken<T>>,
    pub community_issuance: ContractInstance<CommunityIssuance<T>>,
    pub vesting_contract: ContractInstance<VestingContract<T>>,
    pub fpt_asset_id: AssetId,
    pub usdf_asset_id: AssetId,
}

pub struct AssetContracts<T: Account> {
    pub asset: Token<T>,
    pub oracle: ContractInstance<Oracle<T>>,
    pub mock_pyth_oracle: PythCore<T>,
    pub mock_redstone_oracle: RedstoneCore<T>,
    pub trove_manager: ContractInstance<TroveManagerContract<T>>,
    pub asset_id: AssetId,
    pub pyth_price_id: Bits256,
    pub redstone_price_id: U256,
    pub redstone_precision: u32,
    pub fuel_vm_decimals: u32,
    pub symbol: String,
}
pub struct AssetContractsOptionalRedstone<T: Account> {
    pub symbol: String,
    pub asset: Token<T>,
    pub oracle: ContractInstance<Oracle<T>>,
    pub mock_pyth_oracle: PythCore<T>,
    pub trove_manager: ContractInstance<TroveManagerContract<T>>,
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

impl<T: Account> ProtocolContracts<T> {
    pub fn print_contract_ids(&self) {
        println!(
            "Borrow Operations Contract ID: {:?}",
            self.borrow_operations.contract.contract_id()
        );
        println!(
            "Borrow Operations Implementation ID: {:?}",
            self.borrow_operations.implementation_id
        );
        println!(
            "USDF Token Contract ID: {:?}",
            self.usdf.contract.contract_id()
        );
        println!(
            "USDF Token Implementation ID: {:?}",
            self.usdf.implementation_id
        );
        println!(
            "Stability Pool Contract ID: {:?}",
            self.stability_pool.contract.contract_id()
        );
        println!(
            "Protocol Manager Contract ID: {:?}",
            self.protocol_manager.contract.contract_id()
        );
        for asset_contract in &self.asset_contracts {
            println!(
                "Asset Contract ID: {:?}",
                asset_contract.asset.contract_id()
            );
            println!(
                "Oracle Contract ID: {:?}",
                asset_contract.oracle.contract.contract_id()
            );
            println!(
                "Mock Pyth Oracle Contract ID: {:?}",
                asset_contract.mock_pyth_oracle.contract_id()
            );
            println!(
                "Mock Redstone Oracle Contract ID: {:?}",
                asset_contract.mock_redstone_oracle.contract_id()
            );
            println!(
                "Trove Manager Contract ID: {:?}",
                asset_contract.trove_manager.contract.contract_id()
            );
        }
        println!(
            "FPT Staking Contract ID: {:?}",
            self.fpt_staking.contract.contract_id()
        );
        println!(
            "Coll Surplus Pool Contract ID: {:?}",
            self.coll_surplus_pool.contract.contract_id()
        );
        println!(
            "Sorted Troves Contract ID: {:?}",
            self.sorted_troves.contract.contract_id()
        );
        println!(
            "Sorted Troves Implementation ID: {:?}",
            self.sorted_troves.implementation_id
        );
        println!(
            "Default Pool Contract ID: {:?}",
            self.default_pool.contract.contract_id()
        );
        println!(
            "Active Pool Contract ID: {:?}",
            self.active_pool.contract.contract_id()
        );
        println!(
            "FPT Token Contract ID: {:?}",
            self.fpt_token.contract.contract_id()
        );
        println!(
            "Community Issuance Contract ID: {:?}",
            self.community_issuance.contract.contract_id()
        );
        println!(
            "Vesting Contract ID: {:?}",
            self.vesting_contract.contract.contract_id()
        );
        println!(
            "Vesting Implementation ID: {:?}",
            self.vesting_contract.implementation_id
        );
    }
}
