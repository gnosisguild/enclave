// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import {
    IERC20Permit
} from "@openzeppelin/contracts/token/ERC20/extensions/IERC20Permit.sol";
import {
    SafeERC20
} from "@openzeppelin/contracts/token/ERC20/utils/SafeERC20.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import {
    ReentrancyGuard
} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";
import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import {
    CiphernodeRegistryOwnable
} from "../registry/CiphernodeRegistryOwnable.sol";
import { IBondingManager } from "../interfaces/IBondingManager.sol";
import {
    AggregatorV3Interface
} from "@chainlink/contracts/src/v0.8/shared/interfaces/AggregatorV3Interface.sol";

/**
 * @title BondingManager
 * @notice Manages collateral bonding for ciphernodes with USDC and ENCL tokens
 * @dev Requires collateral for ciphernode participation
 */
contract BondingManager is IBondingManager, Ownable, ReentrancyGuard {
    using SafeERC20 for IERC20;

    struct Bond {
        uint256 usdcAmount;
        uint256 enclAmount;
        uint256 totalUsdValue;
        uint256 bondedAt;
        uint256 decommissionRequestedAt;
        bool active;
    }

    /// @notice USDC token contract
    IERC20 public immutable USDC_TOKEN;

    /// @notice ENCL token contract
    IERC20 public immutable ENCL_TOKEN;

    /// @notice CiphernodeRegistry contract
    ICiphernodeRegistry public ciphernodeRegistry;

    /// @notice Minimum bond requirement in USD (18 decimals)
    uint256 public minBondUsd;

    /// @notice Registration delay in seconds
    uint256 public registrationDelay;

    /// @notice Decommission delay in seconds
    uint256 public decommissionDelay;

    /// @notice ENCL price in USD (18 decimals) - manual fallback
    uint256 public enclPriceUsd;

    /// @notice Chainlink price feed for ENCL/USD
    AggregatorV3Interface public enclPriceFeed;

    /// @notice Price feed staleness threshold (24 hours)
    uint256 public constant PRICE_STALENESS_THRESHOLD = 86400;

    /// @notice Mapping of node address to bond information
    mapping(address node => Bond bond) public bonds;

    /// @notice Addresses authorized to slash nodes
    mapping(address slasher => bool authorized) public slashers;

    /**
     * @param _usdc USDC token address
     * @param _encl ENCL token address
     * @param _ciphernodeRegistry Registry address
     * @param _owner Contract owner
     * @param _minBondUsd Minimum bond (18 decimals USD)
     * @param _registrationDelay Registration delay (seconds)
     * @param _decommissionDelay Decommission delay (seconds)
     * @param _enclPriceUsd ENCL price (18 decimals USD)
     */
    constructor(
        address _usdc,
        address _encl,
        address _ciphernodeRegistry,
        address _owner,
        uint256 _minBondUsd,
        uint256 _registrationDelay,
        uint256 _decommissionDelay,
        uint256 _enclPriceUsd
    ) Ownable(_owner) {
        require(_usdc != address(0), ZeroUsdcAddress());
        require(_encl != address(0), ZeroEnclAddress());
        require(_ciphernodeRegistry != address(0), ZeroRegistryAddress());

        USDC_TOKEN = IERC20(_usdc);
        ENCL_TOKEN = IERC20(_encl);
        ciphernodeRegistry = ICiphernodeRegistry(_ciphernodeRegistry);
        minBondUsd = _minBondUsd;
        registrationDelay = _registrationDelay;
        decommissionDelay = _decommissionDelay;
        enclPriceUsd = _enclPriceUsd;
    }

    /**
     * @param usdcAmount USDC amount (6 decimals)
     */
    function bondUSDC(uint256 usdcAmount) external nonReentrant {
        require(usdcAmount > 0, ZeroAmount());

        Bond storage bond = bonds[msg.sender];

        USDC_TOKEN.safeTransferFrom(msg.sender, address(this), usdcAmount);

        bond.usdcAmount += usdcAmount;
        uint256 usdcValueUsd = usdcAmount * 1e12;
        bond.totalUsdValue += usdcValueUsd;

        _handleBondUpdate(bond);

        emit NodeBondedUSDC(msg.sender, usdcAmount, bond.totalUsdValue);
    }

    /**
     * @param enclAmount ENCL amount (18 decimals)
     */
    function bondENCL(uint256 enclAmount) external nonReentrant {
        require(enclAmount > 0, ZeroAmount());

        Bond storage bond = bonds[msg.sender];

        ENCL_TOKEN.safeTransferFrom(msg.sender, address(this), enclAmount);

        // Update bond state
        bond.enclAmount += enclAmount;
        uint256 enclValueUsd = (enclAmount * _getEnclPrice()) / 1e18;
        bond.totalUsdValue += enclValueUsd;

        _handleBondUpdate(bond);

        emit NodeBondedENCL(msg.sender, enclAmount, bond.totalUsdValue);
    }

    /**
     * @param enclAmount ENCL amount (18 decimals)
     * @param deadline Deadline for the permit
     * @param v ECDSA signature
     * @param r ECDSA signature
     * @param s ECDSA signature
     */
    function bondENCLWithPermit(
        uint256 enclAmount,
        uint256 deadline,
        uint8 v,
        bytes32 r,
        bytes32 s
    ) external nonReentrant {
        IERC20Permit(address(ENCL_TOKEN)).permit(
            msg.sender,
            address(this),
            enclAmount,
            deadline,
            v,
            r,
            s
        );
        this.bondENCL(enclAmount);
    }

    /**
     * @notice Request decommission from being a ciphernode
     */
    function requestDecommission() external {
        Bond storage bond = bonds[msg.sender];
        require(bond.active, NotBonded());
        require(bond.decommissionRequestedAt == 0, AlreadyRequested());

        bond.decommissionRequestedAt = block.timestamp;

        emit DecommissionRequested(msg.sender, block.timestamp);
    }

    /**
     * @notice Complete decommission and withdraw collateral
     */
    function completeDecommission(
        uint256[] calldata siblingNodes
    ) external nonReentrant {
        Bond storage bond = bonds[msg.sender];
        require(bond.active, NotBonded());
        require(bond.decommissionRequestedAt > 0, DecommissionNotRequested());
        require(
            block.timestamp >= bond.decommissionRequestedAt + decommissionDelay,
            DecommissionDelayNotPassed()
        );

        uint256 usdcAmount = bond.usdcAmount;
        uint256 enclAmount = bond.enclAmount;

        delete bonds[msg.sender];

        if (usdcAmount > 0) {
            USDC_TOKEN.safeTransfer(msg.sender, usdcAmount);
        }
        if (enclAmount > 0) {
            ENCL_TOKEN.safeTransfer(msg.sender, enclAmount);
        }

        if (address(ciphernodeRegistry) != address(0)) {
            CiphernodeRegistryOwnable reg = CiphernodeRegistryOwnable(
                address(ciphernodeRegistry)
            );
            if (reg.isEnabled(msg.sender)) {
                reg.removeCiphernode(msg.sender, siblingNodes);
            }
        }

        emit NodeDecommissioned(msg.sender, usdcAmount, enclAmount);
    }

    /**
     * @param node Node address
     * @param usdAmount USD amount (18 decimals)
     * @param reason Slash reason
     */
    function slash(
        address node,
        uint256 usdAmount,
        string calldata reason,
        uint256[] calldata siblingNodes
    ) external nonReentrant {
        require(slashers[msg.sender], NotAuthorizedSlasher());
        require(usdAmount > 0, ZeroSlashAmount());

        Bond storage bond = bonds[node];
        require(bond.active, NodeNotBonded());
        require(bond.totalUsdValue >= usdAmount, InsufficientBond());

        uint256 usdcToSlash = 0;
        uint256 enclToSlash = 0;
        uint256 originalUsdAmount = usdAmount;

        if (bond.totalUsdValue > 0) {
            uint256 usdcValueUsd = bond.usdcAmount * 1e12;
            if (usdcValueUsd > 0 && usdAmount > 0) {
                uint256 usdcSlashUsd = usdAmount < usdcValueUsd
                    ? usdAmount
                    : usdcValueUsd;
                usdcToSlash = usdcSlashUsd / 1e12;
                usdAmount -= usdcSlashUsd;
                bond.usdcAmount -= usdcToSlash;
                bond.totalUsdValue -= usdcSlashUsd;
            }

            if (usdAmount > 0 && bond.enclAmount > 0) {
                enclToSlash = (usdAmount * 1e18) / _getEnclPrice();
                if (enclToSlash > bond.enclAmount) {
                    enclToSlash = bond.enclAmount;
                }
                uint256 enclSlashUsd = (enclToSlash * _getEnclPrice()) / 1e18;
                bond.enclAmount -= enclToSlash;
                bond.totalUsdValue -= enclSlashUsd;
            }
        }

        if (usdcToSlash > 0) {
            USDC_TOKEN.safeTransfer(owner(), usdcToSlash);
        }
        if (enclToSlash > 0) {
            ENCL_TOKEN.safeTransfer(owner(), enclToSlash);
        }

        emit NodeSlashed(
            node,
            usdcToSlash,
            enclToSlash,
            originalUsdAmount,
            reason
        );

        if (bond.active && bond.totalUsdValue < minBondUsd) {
            bond.active = false;
            bond.bondedAt = 0;
            emit NodeAtRisk(node, bond.totalUsdValue, minBondUsd);
            if (address(ciphernodeRegistry) != address(0)) {
                CiphernodeRegistryOwnable reg = CiphernodeRegistryOwnable(
                    address(ciphernodeRegistry)
                );
                if (reg.isEnabled(node)) {
                    reg.removeCiphernode(node, siblingNodes);
                }
            }
        }
    }

    /**
     * @param node Node address
     * @return isBonded Bond status
     */
    function isBonded(address node) external view returns (bool) {
        Bond memory bond = bonds[node];
        return
            bond.active &&
            bond.totalUsdValue >= minBondUsd &&
            (bond.bondedAt == 0 ||
                block.timestamp >= bond.bondedAt + registrationDelay);
    }

    /**
     * @notice Get the total USD value of a node's bond
     * @param node Address of the node
     * @return totalUsdValue Total USD value of the node's collateral
     */
    function getBondValue(
        address node
    ) external view returns (uint256 totalUsdValue) {
        return bonds[node].totalUsdValue;
    }

    /**
     * @notice Get detailed bond information for a node
     * @param node Address of the node
     * @return usdcAmount Amount of USDC bonded
     * @return enclAmount Amount of ENCL bonded
     * @return totalUsdValue Total USD value of collateral
     * @return bondedAt Timestamp when node was bonded
     * @return canDecommission Whether node can currently decommission
     */
    function getBondInfo(
        address node
    )
        external
        view
        returns (
            uint256 usdcAmount,
            uint256 enclAmount,
            uint256 totalUsdValue,
            uint256 bondedAt,
            bool canDecommission
        )
    {
        Bond memory bond = bonds[node];
        canDecommission =
            bond.decommissionRequestedAt > 0 &&
            block.timestamp >= bond.decommissionRequestedAt + decommissionDelay;

        return (
            bond.usdcAmount,
            bond.enclAmount,
            bond.totalUsdValue,
            bond.bondedAt,
            canDecommission
        );
    }

    /**
     * @dev Internal bond update logic
     */
    function _handleBondUpdate(Bond storage bond) internal {
        if (!bond.active && bond.totalUsdValue >= minBondUsd) {
            bond.active = true;
            bond.bondedAt = block.timestamp;
        }
        if (
            bond.active &&
            block.timestamp >= bond.bondedAt + registrationDelay &&
            address(ciphernodeRegistry) != address(0)
        ) {
            if (!ciphernodeRegistry.isCiphernodeEligible(msg.sender)) {
                CiphernodeRegistryOwnable(address(ciphernodeRegistry))
                    .addCiphernode(msg.sender);
            }
        }
    }

    // Admin functions

    /**
     * @param _ciphernodeRegistry Registry address
     */
    function setCiphernodeRegistry(
        address _ciphernodeRegistry
    ) external onlyOwner {
        require(_ciphernodeRegistry != address(0), ZeroAddress());
        ciphernodeRegistry = ICiphernodeRegistry(_ciphernodeRegistry);
    }

    /**
     * @notice Set minimum bond requirement
     * @param _minBondUsd New minimum bond in USD (18 decimals)
     */
    function setMinBondUsd(uint256 _minBondUsd) external onlyOwner {
        minBondUsd = _minBondUsd;
        emit MinBondUpdated(_minBondUsd);
    }

    /**
     * @param _registrationDelay Delay in seconds
     */
    function setRegistrationDelay(
        uint256 _registrationDelay
    ) external onlyOwner {
        registrationDelay = _registrationDelay;
        emit RegistrationDelayUpdated(_registrationDelay);
    }

    /**
     * @param _decommissionDelay Delay in seconds
     */
    function setDecommissionDelay(
        uint256 _decommissionDelay
    ) external onlyOwner {
        decommissionDelay = _decommissionDelay;
        emit DecommissionDelayUpdated(_decommissionDelay);
    }

    /**
     * @param _enclPriceUsd ENCL price (18 decimals USD)
     */
    function setEnclPriceUsd(uint256 _enclPriceUsd) external onlyOwner {
        require(_enclPriceUsd > 0, ZeroPrice());
        enclPriceUsd = _enclPriceUsd;
    }

    /**
     * @param _priceFeed Chainlink aggregator address
     */
    function setEnclPriceFeed(address _priceFeed) external onlyOwner {
        enclPriceFeed = AggregatorV3Interface(_priceFeed);
    }

    /**
     * @dev Oracle price with fallback
     */
    function _getEnclPrice() internal view returns (uint256 price) {
        if (address(enclPriceFeed) != address(0)) {
            try enclPriceFeed.latestRoundData() returns (
                uint80 /* roundId */,
                int256 answer,
                uint256 /* startedAt */,
                uint256 updatedAt,
                uint80 /* answeredInRound */
            ) {
                // Check if price is positive and not stale
                if (
                    answer > 0 &&
                    block.timestamp - updatedAt <= PRICE_STALENESS_THRESHOLD
                ) {
                    return uint256(answer) * 1e10;
                } else {
                    // Oracle value is stale or invalid, fallback
                    return enclPriceUsd;
                }
            } catch {
                // Oracle call failed, fallback
                return enclPriceUsd;
            }
        }
        // No price feed configured, fallback
        return enclPriceUsd;
    }

    /**
     * @return price ENCL price (18 decimals USD)
     */
    function getEnclPrice() external view returns (uint256 price) {
        return _getEnclPrice();
    }

    /**
     * @param slasher Slasher address
     */
    function addSlasher(address slasher) external onlyOwner {
        require(slasher != address(0), ZeroAddress());
        slashers[slasher] = true;
        emit SlasherAdded(slasher);
    }

    /**
     * @param slasher Slasher address
     */
    function removeSlasher(address slasher) external onlyOwner {
        slashers[slasher] = false;
        emit SlasherRemoved(slasher);
    }

    /**
     * @param token Token address
     * @param amount Withdraw amount
     */
    function emergencyWithdraw(
        address token,
        uint256 amount
    ) external onlyOwner {
        IERC20(token).safeTransfer(owner(), amount);
    }
}
