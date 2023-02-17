library interface;

dep data_structures;

use data_structures::VestingSchedule;

abi VestingContract {
    #[storage(write)]
    fn set_price(price: u64);

    #[storage(read)]
    fn get_price() -> u64;

    #[storage(read)]
    fn get_precision() -> u64;

    #[storage(read)]
    fn get_vesting_schedules() -> Vec<VestingSchedule>;
}