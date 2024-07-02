library;

abi Oracle {
    // TODO: remove
    #[storage(write)]
    fn set_price(price: u64);

    // TODO: return Price
    #[storage(read, write)]
    fn get_price() -> u64;

    // TODO: remove?
    #[storage(read)]
    fn get_precision() -> u64;
}

// Placeholder for oracle integration
abi OracleModule {
    #[storage(read)]
    fn price() -> Price;

    #[storage(write)]
    fn set_module_price(price: u64);
}


pub struct Price {
    value: u64,
    time: u64
}

impl Price {
    pub fn new(price: u64, time: u64) -> Self {
        Self {
            value: price,
            time
        }
    }
}