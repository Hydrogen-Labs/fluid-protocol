# Fluid Protocol

## [`Good Sway Contract Reference`](https://github.com/FuelLabs/sway-applications/tree/master/AMM/project)

Contracts
---------

The source code for each contract is in the [`contracts/`](contracts/)
directory.

| Name                                               | Description                            |
| -------------------------------------------------- | -------------------------------------- |
| [`mock-oracle`](contracts/mock-oracle-contract)       | Oracle for on-chain data |
| [`trove-manager`](contracts/trove-manager-contract)       | Manages minting $USDF, liquidations, and user troves |
| [`stability-pool`](contracts/stability-pool-contract)       | Manages desposits to liquidate user troves |
| [`vesting`](contracts/vesting-contract)       | Manages $FPT vesting schedules |

Build + Test Contracts
-------------------------------

Make sure you have fuelup, fuel-core, cargo, and rust installed 

```
sh build-and-test.sh
```

License
-------

MIT License (see `/LICENSE`)
