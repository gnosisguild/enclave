// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

interface IStrategy {
    function underlyingToken() external view returns (IERC20);

    function sharesToUnderlying(
        uint256 amountShares
    ) external view returns (uint256);

    function underlyingToShares(
        uint256 amountUnderlying
    ) external view returns (uint256);

    function deposit(
        IERC20 token,
        uint256 amount
    ) external returns (uint256 newShares);
}
