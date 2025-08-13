// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import {
    ServiceManagerBase
} from "../../lib/eigenlayer-middleware/src/ServiceManagerBase.sol";
import {
    IRegistryCoordinator
} from "../../lib/eigenlayer-middleware/src/interfaces/IRegistryCoordinator.sol";
import {
    IStakeRegistry
} from "../../lib/eigenlayer-middleware/src/interfaces/IStakeRegistry.sol";
import {
    IStrategy
} from "../../lib/eigenlayer-contracts/src/contracts/interfaces/IStrategy.sol";
import {
    IAllocationManager
} from "../../lib/eigenlayer-contracts/src/contracts/interfaces/IAllocationManager.sol";
import {
    AggregatorV3Interface
} from "@chainlink/contracts/src/v0.8/shared/interfaces/AggregatorV3Interface.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import {
    ReentrancyGuard
} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { IBondingManager } from "../interfaces/IBondingManager.sol";

contract EnclaveServiceManager is ServiceManagerBase, Ownable, ReentrancyGuard {
    /// @notice Strategy configuration
    struct StrategyConfig {
        bool isAllowed;
        uint256 minShares;
        address priceFeed;
        uint8 decimals;
    }

    event CiphernodeRegistered(address indexed operator, uint256 collateralUsd);
    event CiphernodeDeregistered(address indexed operator);
    event StrategyAdded(
        IStrategy indexed strategy,
        uint256 minShares,
        address priceFeed
    );
    event StrategyRemoved(IStrategy indexed strategy);
    event StrategyUpdated(
        IStrategy indexed strategy,
        uint256 newMinShares,
        address newPriceFeed
    );
    event MinCollateralUpdated(uint256 newMinCollateralUsd);

    uint256 public minCollateralUsd;
    mapping(IStrategy => StrategyConfig) public strategyConfigs;
    IStrategy[] public allowedStrategies;
    mapping(IStrategy => uint256) private strategyToIndex;

    ICiphernodeRegistry public ciphernodeRegistry;
    IBondingManager public bondingManager;
    IAllocationManager public allocationManager;

    mapping(address => bool) public registeredOperators;
    uint32 public operatorSetId;

    /// @notice Minimum collateral requirement in USD
    uint256 public minCollateralUsd;

    /// @notice Supported strategies mapping
    mapping(IStrategy => StrategyConfig) public strategyConfigs;

    /// @notice Array of all allowed strategies
    IStrategy[] public allowedStrategies;

    /// @notice Mapping of strategy to its index in allowedStrategies array
    mapping(IStrategy => uint256) private strategyToIndex;

    /// @notice CiphernodeRegistry contract
    ICiphernodeRegistry public ciphernodeRegistry;

    /// @notice EigenLayer BondingManager contract
    IBondingManager public bondingManager;

    /// @notice EigenLayer AllocationManager for slashing
    IAllocationManager public allocationManager;

    /// @notice Registered operators
    mapping(address => bool) public registeredOperators;

    /// @notice Operator set ID for slashing
    uint32 public operatorSetId;

    constructor(
        IAVSDirectory _avsDirectory,
        IRegistryCoordinator _registryCoordinator,
        IStakeRegistry _stakeRegistry,
        IAllocationManager _allocationManager,
        address _ciphernodeRegistry,
        address _bondingManager,
        address _owner,
        uint256 _minCollateralUsd,
        uint32 _operatorSetId
    )
        ServiceManagerBase(_avsDirectory, _registryCoordinator, _stakeRegistry)
        Ownable(_owner)
    {
        require(_ciphernodeRegistry != address(0), "zero ciphernodeRegistry");
        require(_bondingManager != address(0), "zero bondingManager");
        require(
            address(_allocationManager) != address(0),
            "zero allocationMgr"
        );
        require(_minCollateralUsd > 0, "invalid min collateral");

        ciphernodeRegistry = ICiphernodeRegistry(_ciphernodeRegistry);
        bondingManager = IBondingManager(_bondingManager);
        allocationManager = _allocationManager;
        minCollateralUsd = _minCollateralUsd;
        operatorSetId = _operatorSetId;
    }

    // ============ Strategy Management ============

    function addStrategy(
        IStrategy strategy,
        uint256 minShares,
        address priceFeed
    ) external onlyOwner {
        require(address(strategy) != address(0), "zero strategy");
        require(!strategyConfigs[strategy].isAllowed, "already allowed");

        strategyConfigs[strategy] = StrategyConfig({
            isAllowed: true,
            minShares: minShares,
            priceFeed: priceFeed,
            decimals: 18
        });

        strategyToIndex[strategy] = allowedStrategies.length;
        allowedStrategies.push(strategy);
    }

    function removeStrategy(IStrategy strategy) external onlyOwner {
        require(strategyConfigs[strategy].isAllowed, "strategy not found");

        uint256 index = strategyToIndex[strategy];
        uint256 lastIndex = allowedStrategies.length - 1;

        if (index != lastIndex) {
            IStrategy lastStrategy = allowedStrategies[lastIndex];
            allowedStrategies[index] = lastStrategy;
            strategyToIndex[lastStrategy] = index;
        }

        allowedStrategies.pop();
        delete strategyToIndex[strategy];
        delete strategyConfigs[strategy];
    }

    function updateStrategy(
        IStrategy strategy,
        uint256 newMinShares,
        address newPriceFeed
    ) external onlyOwner {
        require(strategyConfigs[strategy].isAllowed, "strategy not found");

        strategyConfigs[strategy].minShares = newMinShares;
        strategyConfigs[strategy].priceFeed = newPriceFeed;
    }

    function setMinCollateralUsd(uint256 _minCollateralUsd) external onlyOwner {
        require(_minCollateralUsd > 0, "invalid min collateral");
        minCollateralUsd = _minCollateralUsd;
    }

    function registerCiphernode() external nonReentrant {
        require(!registeredOperators[msg.sender], "already registered");

        // must be an EigenLayer operator
        require(_delegationManager.isOperator(msg.sender), "not EL operator");

        // meet collateral
        (bool ok, uint256 collateralUsd) = checkOperatorEligibility(msg.sender);
        require(ok, "insufficient collateral");

        registeredOperators[msg.sender] = true;

        // reflect in bonding + registry
        bondingManager.registerOperator(msg.sender, collateralUsd);
        ciphernodeRegistry.addCiphernode(msg.sender);

        emit CiphernodeRegistered(msg.sender, collateralUsd);
    }

    function deregisterCiphernode(
        uint256[] calldata siblingNodes
    ) external nonReentrant {
        require(registeredOperators[msg.sender], "not registered");

        registeredOperators[msg.sender] = false;

        bondingManager.deregisterOperator(msg.sender);
        ciphernodeRegistry.removeCiphernode(msg.sender, siblingNodes);

        emit CiphernodeDeregistered(msg.sender);
    }

    // ============ Slashing ============

    function addSlasher(address slasher) external onlyOwner {
        require(slasher != address(0), "zero slasher");
        slashers[slasher] = true;
    }

    function removeSlasher(address slasher) external onlyOwner {
        slashers[slasher] = false;
    }

    function slashOperator(
        address operator,
        uint256 slashingPercentage, // bps
        string calldata reason
    ) external nonReentrant {
        require(slashers[msg.sender], "not slasher");
        require(registeredOperators[operator], "operator not registered");
        require(slashingPercentage <= 10000, "bps too high");

        (
            IStrategy[] memory strategies,
            uint256[] memory wadsToSlash
        ) = _calculateSlashingWads(operator, slashingPercentage);

        if (strategies.length == 0) {
            return;
        }

        try
            allocationManager.slashOperator(
                address(this),
                IAllocationManager.SlashingParams({
                    operator: operator,
                    operatorSetId: operatorSetId,
                    strategies: strategies,
                    wadsToSlash: wadsToSlash,
                    description: reason
                })
            )
        returns (uint256 /*slashId*/, uint256[] memory slashedShares) {
            emit OperatorSlashed(
                operator,
                slashingPercentage,
                reason,
                strategies,
                slashedShares
            );

            (bool ok, uint256 newCollateralUsd) = checkOperatorEligibility(
                operator
            );
            if (!ok) {
                registeredOperators[operator] = false;
                bondingManager.deregisterOperator(operator);
                emit CiphernodeDeregistered(operator);
            } else {
                bondingManager.updateOperatorCollateral(
                    operator,
                    newCollateralUsd
                );
            }
        } catch Error(string memory err) {
            revert(string(abi.encodePacked("slashing failed: ", err)));
        }
    }

    function _calculateSlashingWads(
        address operator,
        uint256 slashingPercentage
    )
        internal
        view
        returns (IStrategy[] memory strategies, uint256[] memory wadsToSlash)
    {
        uint256 count = 0;
        for (uint256 i = 0; i < allowedStrategies.length; i++) {
            if (getOperatorShares(operator, allowedStrategies[i]) > 0) {
                count++;
            }
        }
        strategies = new IStrategy[](count);
        wadsToSlash = new uint256[](count);

        uint256 idx = 0;
        uint256 wad = (slashingPercentage * 1e18) / 10000; // bps -> wad
        for (uint256 i = 0; i < allowedStrategies.length; i++) {
            IStrategy s = allowedStrategies[i];
            if (getOperatorShares(operator, s) > 0) {
                strategies[idx] = s;
                wadsToSlash[idx] = wad;
                idx++;
            }
        }
    }

    function checkOperatorEligibility(
        address operator
    ) public view returns (bool isEligible, uint256 collateralUsd) {
        collateralUsd = getOperatorCollateralValue(operator);
        isEligible = collateralUsd >= minCollateralUsd;
    }

    function getOperatorCollateralValue(
        address operator
    ) public view returns (uint256 totalUsdValue) {
        for (uint256 i = 0; i < allowedStrategies.length; i++) {
            IStrategy strategy = allowedStrategies[i];
            uint256 shares = getOperatorShares(operator, strategy);
            if (shares > 0) {
                totalUsdValue += _convertSharesToUsd(strategy, shares);
            }
        }
    }

    function getOperatorShares(
        address operator,
        IStrategy strategy
    ) public view returns (uint256 shares) {
        return _strategyManager.stakerStrategyShares(operator, strategy);
    }

    function isStrategyAllowed(
        IStrategy strategy
    ) external view returns (bool isAllowed) {
        return strategyConfigs[strategy].isAllowed;
    }

    function getAllowedStrategies()
        external
        view
        returns (IStrategy[] memory strategies)
    {
        return allowedStrategies;
    }

    function getStrategyConfig(
        IStrategy strategy
    ) external view returns (uint256 minShares, address priceFeed) {
        StrategyConfig memory config = strategyConfigs[strategy];
        return (config.minShares, config.priceFeed);
    }

    function getMinCollateralUsd() external view returns (uint256) {
        return minCollateralUsd;
    }

    // ============ Internal ============

    function _convertSharesToUsd(
        IStrategy strategy,
        uint256 shares
    ) internal view returns (uint256 usdValue) {
        StrategyConfig memory config = strategyConfigs[strategy];
        uint256 underlyingAmount = strategy.sharesToUnderlyingView(shares);

        if (config.priceFeed == address(0)) {
            usdValue = underlyingAmount;
        } else {
            uint256 price = _getTokenPrice(config.priceFeed);
            usdValue = (underlyingAmount * price) / 1e8;
        }
    }

    function _getTokenPrice(
        address priceFeed
    ) internal view returns (uint256 price) {
        AggregatorV3Interface feed = AggregatorV3Interface(priceFeed);
        (, int256 answer, , , ) = feed.latestRoundData();
        require(answer > 0, "bad price");
        return uint256(answer);
    }
}
