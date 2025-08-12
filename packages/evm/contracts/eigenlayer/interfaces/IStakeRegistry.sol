// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

interface IStakeRegistry {
    function stakerStrategyShares(
        address staker,
        address strategy
    ) external view returns (uint256);
}
