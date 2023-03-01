library data_structures;

pub struct Trove {
    usdf_borrowed: u64,
    fuel_deposited: u64,
    st_fuel_deposited: u64,
}

pub enum Status {
    NonExistent: (),
    Active: (),
    ClosedByOwner: (),
    ClosedByLiquidation: (),
    ClosedByRedemption: (),
}

impl Status {
    pub fn eq(self, other: Status) -> bool {
        match (self, other) {
            (Status::NonExistent, Status::NonExistent) => true,
            (Status::Active, Status::Active) => true,
            (Status::ClosedByOwner, Status::ClosedByOwner) => true,
            (Status::ClosedByLiquidation, Status::ClosedByLiquidation) => true,
            (Status::ClosedByRedemption, Status::ClosedByRedemption) => true,
            _ => false,
        }
    }

   pub fn neq(self, other: Status) -> bool {
        match (self, other) {
            (Status::NonExistent, Status::NonExistent) => false,
            (Status::Active, Status::Active) => false,
            (Status::ClosedByOwner, Status::ClosedByOwner) => false,
            (Status::ClosedByLiquidation, Status::ClosedByLiquidation) => false,
            (Status::ClosedByRedemption, Status::ClosedByRedemption) => false,
            _ => true,
        }
    }
}

pub struct Asset {
    /// Identifier of asset
    id: ContractId,
    /// Amount of asset that can represent reserve amount, deposit amount, withdraw amount and more depending on the context
    amount: u64,
}

pub struct Node {
    exists: bool,
    next_id: Identity,
    prev_id: Identity,
}

impl Asset {
    pub fn new(id: ContractId, amount: u64) -> Self {
        Self { id, amount }
    }
}
