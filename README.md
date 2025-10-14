# Fluid Protocol

Fluid is a decentralized protocol that allows holders of certain Assets to obtain maximum liquidity against
their collateral without paying interest. After locking up their assets as collateral in a smart contract and
creating an individual position called a "trove", the user can get instant liquidity by minting USDF,
a USD-pegged stablecoin. Each trove is required to be collateralized at a minimum of 135%. Any
owner of USDF can redeem their stablecoins for the underlying collateral at any time.

An unprecedented liquidation mechanism based on incentivized stability deposits and a redistribution
cycle from riskier to safer troves provides stability at a much lower collateral ratio than current
systems. Stability is maintained via economically-driven user interactions and arbitrage, rather
than by active governance or monetary interventions.

## Contracts

The source code for each contract is in the [`contracts/`](contracts/)
directory.

| Name                                                          | Description                                                                                                                 |
| ------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| [`protocol-manager`](contracts/protocol-manager-contract)     | Proxy for adding new assets as collateral, and manages USDF redemptions, ownership to be renounced after milestones reached |
| [`borrow-operations`](contracts/borrow-operations-contract)   | Interface with which users manager their troves                                                                             |
| [`stability-pool`](contracts/stability-pool-contract)         | Manages $USDF desposits to liquidate user troves                                                                            |
| [`USDF-token`](contracts/usdf-token-contract)                 | Token Contract for authorizing minting,burning of $USDF                                                                     |
| [`active-pool`](contracts/active-pool-contract)               | Central place for holding collateral from Active Troves                                                                     |
| [`community-issuance`](contracts/community-issuance-contract) | Manages the issuance of community rewards                                                                                   |
| [`default-pool`](contracts/default-pool-contract)             | Central place for holding 'unapplied' rewards from liquidation redistributions                                              |
| [`coll-surplus-pool`](contracts/coll-surplus-pool-contract)   | Central place for holding exess assets from either a redemption or a full liquidation                                       |
| [`sorted-troves`](contracts/sorted-troves-contract)           | Manages location of troves in the Linked list format                                                                        |
| Asset Specific Contracts                                      |
| [`token`](contracts/token-contract)                           | FRC-20 to use in local tests made by Sway Gang                                                                              |
| [`oracle`](contracts/oracle-contract)                         | Oracle for on-chain data                                                                                                    |
| [`trove-manager`](contracts/trove-manager-contract)           | Manages liquidations, redemptions, and user troves in the Linked list format                                                |
| FPT Contracts                                                 |
| [`FPT-vesting`](contracts/vesting-contract)                   | Manages $FPT vesting schedules                                                                                              |
| [`FPT-staking`](contracts/staking-contract)                   | Manages $FPT staking emissions from fee collection                                                                          |
| [`FPT-token`](contracts/fpt-token-contract)                   | Token contract for the FPT (Fluid Protocol Token)                                                                           |

## Dependencies

- rust 1.80.1
- [Fuel](https://fuel.network/)
- [Wasm pack](https://github.com/FuelLabs/fuels-rs?tab=readme-ov-file#how-to-run-wasm-tests)

## Build + Test Contracts

Make sure you have fuelup, fuel-core, cargo, and rust installed

```
make build-and-test
```

## Functionality

- ✅ Create Trove and Recieve $USDF
- ✅ Add more collateral to trove
- ✅ Add more debt to trove
- ✅ Repay trove debt
- ✅ Reduce collateral from trove
- ✅ Close Trove
- ✅ Liquidate Troves
- ✅ Stability Pool
- ✅ Multiple assets
- ✅ Fees
- ✅ Redeem Collateral w/ USDF
- ✅ Stake FPT

## License

MIT License (see `/LICENSE`)

## More information

Visit [fluid.io](https://fluid.io) to find out more and join the discussion.

## Disclaimer

The content of this readme document (“Readme”) is of purely informational nature. In particular, none of the content of the Readme shall be understood as advice provided by Fluid AG, any Fluid Project Team member or other contributor to the Readme, nor does any of these persons warrant the actuality and accuracy of the Readme.

Please read this Disclaimer carefully before accessing, interacting with, or using the Fluid Protocol software, consisting of the Fluid Protocol technology stack (in particular its smart contracts).

While Fluid AG developed the Fluid Protocol Software, the Fluid Protocol Software runs in a fully decentralized and autonomous manner on the Fuel network. Fluid AG is not involved in the operation of the Fluid Protocol Software nor has it any control over transactions made using its smart contracts. Further, Fluid AG does neither enter into any relationship with users of the Fluid Protocol Software and/or frontend operators, nor does it operate an own frontend. Any and all functionalities of the Fluid Protocol Software, including the USDF and the FPT, are of purely technical nature and there is no claim towards any private individual or legal entity in this regard.

Fluid AG IS NOT LIABLE TO ANY USER FOR DAMAGES, INCLUDING ANY GENERAL, SPECIAL, INCIDENTAL OR CONSEQUENTIAL DAMAGES ARISING OUT OF THE USE, IN CONNECTION WITH THE USE OR INABILITY TO USE THE Fluid PROTOCOL SOFTWARE (INCLUDING BUT NOT LIMITED TO LOSS OF ETH, USDF OR FPT, NON-ALLOCATION OF TECHNICAL FEES TO FPT HOLDERS, LOSS OF DATA, BUSINESS INTERRUPTION, DATA BEING RENDERED INACCURATE OR OTHER LOSSES SUSTAINED BY A USER OR THIRD PARTIES AS A RESULT OF THE Fluid PROTOCOL SOFTWARE AND/OR ANY ACTIVITY OF A FRONTEND OPERATOR OR A FAILURE OF THE Fluid PROTOCOL SOFTWARE TO OPERATE WITH ANY OTHER SOFTWARE).

The Fluid Protocol Software has been developed and published under the GNU GPL v3 open-source license, which forms an integral part of this disclaimer.

THE Fluid PROTOCOL SOFTWARE HAS BEEN PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. THE Fluid PROTOCOL SOFTWARE IS HIGHLY EXPERIMENTAL AND ANY REAL Assets AND/OR USDF AND/OR FPT SENT, STAKED OR DEPOSITED TO THE Fluid PROTOCOL SOFTWARE ARE AT RISK OF BEING LOST INDEFINITELY, WITHOUT ANY KIND OF CONSIDERATION.

There are no official frontend operators, and the use of any frontend is made by users at their own risk. To assess the trustworthiness of a frontend operator lies in the sole responsibility of the users and must be made carefully.

User is solely responsible for complying with applicable law when interacting (in particular, when using ETH, USDF, FPT or other Token) with the Fluid Protocol Software whatsoever.
