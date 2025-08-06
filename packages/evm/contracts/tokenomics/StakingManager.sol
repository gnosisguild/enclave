// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import {
    ReentrancyGuard
} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

contract StakingManager is Ownable, ReentrancyGuard {
    struct Stake {
        uint256 usdcAmount;
        uint256 enclAmount;
        uint256 totalUsdValue;
        uint256 bondedAt;
        uint256 decommissionRequestedAt;
        bool active;
    }

    mapping(address => Stake) public stakes;

    IERC20 public immutable USDC_TOKEN;
    IERC20 public immutable ENCL_TOKEN;

    constructor(address _usdc, address _encl, address _owner) Ownable(_owner) {
        require(_usdc != address(0), "Zero USDC address");
        require(_encl != address(0), "Zero ENCL address");

        USDC_TOKEN = IERC20(_usdc);
        ENCL_TOKEN = IERC20(_encl);
    }

    function bondUSDC(uint256 usdcAmount) external nonReentrant {
        require(usdcAmount > 0, "Zero amount");

        Stake storage stake = stakes[msg.sender];
        USDC_TOKEN.transferFrom(msg.sender, address(this), usdcAmount);

        stake.usdcAmount += usdcAmount;
        stake.totalUsdValue += usdcAmount * 1e12;
    }

    function bondENCL(uint256 enclAmount) external nonReentrant {
        require(enclAmount > 0, "Zero amount");

        Stake storage stake = stakes[msg.sender];
        ENCL_TOKEN.transferFrom(msg.sender, address(this), enclAmount);

        stake.enclAmount += enclAmount;
    }
}
