// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import "@openzeppelin/contracts/access/Ownable.sol";
import "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import "@openzeppelin/contracts/token/ERC20/IERC20.sol";

contract VestingEscrow is Ownable, ReentrancyGuard {
    struct VestingStream {
        uint256 totalAmount;
        uint256 startTime;
        uint256 vestingDuration;
    }

    IERC20 public immutable token;
    mapping(address => VestingStream) public vestingStreams;
    uint256 public totalEscrowed;
    uint256 public totalClaimed;

    constructor(address _token, address _owner) Ownable(_owner) {
        require(_token != address(0), "Zero token address");
        token = IERC20(_token);
    }
}
