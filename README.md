# Fluid Protocol

## [`Good Sway Contract Reference`](https://github.com/FuelLabs/sway-applications/tree/master/AMM/project)

Contracts
---------

The source code for each contract is in the [`contracts/`](contracts/)
directory.

| Name                                               | Description                            | Status |
| -------------------------------------------------- | -------------------------------------- | ------- |
| [`mock-oracle`](contracts/mock-oracle-contract)       | Oracle for on-chain data | $$\color{green}{90/100}$$
| [`active-pool`](contracts/active-pool-contract)       | Central place for holding collateral from Active Troves | $$\color{green}{90/100}$$
| [`default-pool`](contracts/default-pool-contract)       | Central place for holding 'unapplied' rewards from liquidation redistributions | $$\color{green}{90/100}$$
| [`sorted-troves`](contracts/sorted-troves-contract)       | Manages location of troves in the Linked list format | $$\color{green}{90/100}$$
| [`borrow-operations`](contracts/borrow-operations-contract)   | Interface with which users manager their troves | $$\color{green}{90/100}$$ |
| [`trove-manager`](contracts/trove-manager-contract)       | Manages minting $USDF, liquidations, and user troves in the Linked list format |$$\color{orange}{70/100}$$
| [`stability-pool`](contracts/stability-pool-contract)       | Manages $USDF desposits to liquidate user troves | $$\color{orange}{50/100}$$
| [`protocol-factory`](contracts/protocol-contract)       | Routes risk functions to riskies trove from all trove managers, instatiates everything | $$\color{red}{0/100}$$
| [`token`](contracts/token-contract)       | FRC-20 to use in local tests made by Sway Gang | $$\color{green}{90/100}$$
| [`FPT-vesting`](contracts/vesting-contract)       | Manages $FPT vesting schedules | $$\color{orange}{85/100}$$
| [`FPT-staking`](contracts/staking-contract)       | Manages $FPT staking emissions from fee collection | $$\color{orange}{50/100}$$ |

Build + Test Contracts
-------------------------------

Make sure you have fuelup, fuel-core, cargo, and rust installed 

```
sh build-and-test.sh
```

Functionality
-------------------------------
- [x] Create Trove and Recieve $USDF
- [x] Add more collateral to trove
- [x] Add more debt to trove
- [x] Repay trove debt 
- [x] Reduce collateral from trove
- [x] Close Trove
- [x] Liquidate Troves
- [ ] Stability Pool
- [ ] Redeem Fuel/stFuel w/ USDF
- [ ] Multiple assets (Fuel, stFuel)
- [ ] Fees
- [ ] Stake FPT

License
-------

MIT License (see `/LICENSE`)
