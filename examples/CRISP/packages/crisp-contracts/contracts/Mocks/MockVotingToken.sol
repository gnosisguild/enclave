// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity ^0.8.27;

import { ERC20 } from "@openzeppelin/contracts/token/ERC20/ERC20.sol";

/// @title MockVotingToken
/// @notice A mock voting token for testing purposes
/// @dev Public mint function that allows to keep balances to 1e9. 
/// @dev by default CRISP server will scale down voting power by 1e18/2 
/// @dev in this case leaving everyone with a balance of 1 to vote yes or no
contract MockVotingToken is ERC20 {
    // half of 10e18
    uint256 public constant MAX_BALANCE = 1e9;

    constructor() ERC20("Mock Voting Token", "MVT") {
        _mint(msg.sender, 1e9);
    }

    function mint(address to, uint256) external {
        if (balanceOf(to) + 1e9 > MAX_BALANCE) {
            // silently fail 
            return;
        }
        _mint(to, 1e9);
    }

    function decimals() public pure override returns (uint8) {
        return 18;
    }
}