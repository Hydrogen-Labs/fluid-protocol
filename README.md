# Fluid Protocol

## [`Good Sway Contract Reference`](https://github.com/FuelLabs/sway-applications/tree/master/AMM/project)

Contracts
---------

The source code for each contract is in the [`contracts/`](contracts/)
directory.

| Name                                               | Description                            | Status |
| -------------------------------------------------- | -------------------------------------- | ------- |
| [`protocol-manager`](contracts/protocol-manager-contract)       | Proxy for adding new assets as collateral, and manages USDF redemptions, ownership to be renounced after milestones reached | $$\color{green}{85/100}$$
| [`borrow-operations`](contracts/borrow-operations-contract)   | Interface with which users manager their troves | $$\color{green}{90/100}$$ |
| [`stability-pool`](contracts/stability-pool-contract)       | Manages $USDF desposits to liquidate user troves | $$\color{orange}{75/100}$$
| [`USDF-token`](contracts/usdf-token-contract)       | Token Contract for authorizing minting,burning of $USDF | $$\color{green}{90/100}$$
| Asset Specific Contracts                              |
| [`token`](contracts/token-contract)       | FRC-20 to use in local tests made by Sway Gang | $$\color{green}{90/100}$$
| [`mock-oracle`](contracts/mock-oracle-contract)       | Oracle for on-chain data | $$\color{green}{90/100}$$
| [`active-pool`](contracts/active-pool-contract)       | Central place for holding collateral from Active Troves | $$\color{green}{90/100}$$
| [`default-pool`](contracts/default-pool-contract)       | Central place for holding 'unapplied' rewards from liquidation redistributions | $$\color{green}{90/100}$$
| [`coll-surplus-pool`](contracts/coll-surplus-pool-contract)       | Central place for holding exess assets from either a redemption or a full liquidation | $$\color{orange}{80/100}$$
| [`sorted-troves`](contracts/sorted-troves-contract)       | Manages location of troves in the Linked list format | $$\color{green}{90/100}$$
| [`trove-manager`](contracts/trove-manager-contract)       | Manages liquidations, redemptions, and user troves in the Linked list format |$$\color{orange}{70/100}$$
|  FPT Contracts |
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
- [x] Stability Pool
- [x] Multiple assets (Fuel, stFuel)
- [x] Fees
- [x] Redeem Collateral w/ USDF
- [ ] Stake FPT

![Screen Shot 2023-03-30 at 14 45 14](https://user-images.githubusercontent.com/32445955/228947561-0c6b46c9-93bc-4409-a8c6-90199ce46c4f.png)


License
-------

MIT License (see `/LICENSE`)
