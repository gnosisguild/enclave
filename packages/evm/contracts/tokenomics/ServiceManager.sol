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
    IAVSDirectory
} from "../../lib/eigenlayer-contracts/src/contracts/interfaces/IAVSDirectory.sol";
import {
    IStrategyManager
} from "../../lib/eigenlayer-contracts/src/contracts/interfaces/IStrategyManager.sol";
import {
    AggregatorV3Interface
} from "@chainlink/contracts/src/v0.8/shared/interfaces/AggregatorV3Interface.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import {
    ReentrancyGuard
} from "@openzeppelin/contracts/utils/ReentrancyGuard.sol";

import { IServiceManager } from "../interfaces/IServiceManager.sol";
import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { IBondingManager } from "../interfaces/IBondingManager.sol";

contract ServiceManager is
    ServiceManagerBase,
    IServiceManager,
    Ownable,
    ReentrancyGuard
{
    /// @notice Minimum collateral requirement in USD
    uint256 public minCollateralUsd;

    /// @notice Price feed staleness threshold (24 hours)
    uint256 public constant PRICE_STALENESS_THRESHOLD = 86400;

    /// @notice Supported strategies mapping
    mapping(IStrategy strategy => StrategyConfig config) public strategyConfigs;

    /// @notice Array of all allowed strategies
    IStrategy[] public allowedStrategies;

    /// @notice Mapping of strategy to its index in allowedStrategies array
    mapping(IStrategy strategy => uint256 index) private strategyToIndex;

    /// @notice CiphernodeRegistry contract
    ICiphernodeRegistry public ciphernodeRegistry;

    /// @notice EigenLayer BondingManager contract
    IBondingManager public bondingManager;

    /// @notice EigenLayer AllocationManager for slashing
    IAllocationManager public allocationManager;

    /// @notice EigenLayer StrategyManager for share queries
    IStrategyManager public strategyManager;

    /// @notice Addresses authorized to slash operators
    mapping(address slasher => bool isAuthorized) public slashers;

    /// @notice Registered operators
    mapping(address operator => bool isRegistered) public registeredOperators;

    /// @notice Operator set ID
    uint32 public operatorSetId;

    /// @notice Mapping of operators to their allocated magnitudes per strategy
    mapping(address operator => mapping(IStrategy strategy => uint256 allocatedMagnitude))
        public operatorAllocatedMagnitudes;

    constructor(
        IAVSDirectory _avsDirectory,
        IRegistryCoordinator _registryCoordinator,
        IStakeRegistry _stakeRegistry,
        IAllocationManager _allocationManager,
        IStrategyManager _strategyManager,
        address _ciphernodeRegistry,
        address _bondingManager,
        address _owner,
        uint256 _minCollateralUsd,
        uint32 _operatorSetId
    )
        ServiceManagerBase(_avsDirectory, _registryCoordinator, _stakeRegistry)
        Ownable(_owner)
    {
        require(_ciphernodeRegistry != address(0), ZeroAddress());
        require(_bondingManager != address(0), ZeroAddress());
        require(address(_allocationManager) != address(0), ZeroAddress());
        require(address(_strategyManager) != address(0), ZeroAddress());
        require(_minCollateralUsd > 0, InvalidMinCollateral());

        ciphernodeRegistry = ICiphernodeRegistry(_ciphernodeRegistry);
        bondingManager = IBondingManager(_bondingManager);
        allocationManager = _allocationManager;
        strategyManager = _strategyManager;
        minCollateralUsd = _minCollateralUsd;
        operatorSetId = _operatorSetId;
    }

    // ============ Strategy Management ============

    function addStrategy(
        IStrategy strategy,
        uint256 minShares,
        address priceFeed
    ) external onlyOwner {
        require(address(strategy) != address(0), ZeroAddress());
        require(!strategyConfigs[strategy].isAllowed, StrategyAlreadyAllowed());

        uint8 decimals = 18;
        try strategy.underlyingToken().decimals() returns (uint8 d) {
            decimals = d;
        } catch {
            revert("Strategy decimals not found");
        }

        strategyConfigs[strategy] = StrategyConfig({
            isAllowed: true,
            minShares: minShares,
            priceFeed: priceFeed,
            decimals: decimals
        });

        strategyToIndex[strategy] = allowedStrategies.length;
        allowedStrategies.push(strategy);

        emit StrategyAdded(strategy, minShares, priceFeed);
    }

    function removeStrategy(IStrategy strategy) external onlyOwner {
        require(strategyConfigs[strategy].isAllowed, StrategyNotFound());

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

        emit StrategyRemoved(strategy);
    }

    function updateStrategy(
        IStrategy strategy,
        uint256 newMinShares,
        address newPriceFeed
    ) external onlyOwner {
        require(strategyConfigs[strategy].isAllowed, StrategyNotFound());

        strategyConfigs[strategy].minShares = newMinShares;
        strategyConfigs[strategy].priceFeed = newPriceFeed;

        emit StrategyUpdated(strategy, newMinShares, newPriceFeed);
    }

    function setMinCollateralUsd(uint256 _minCollateralUsd) external onlyOwner {
        require(_minCollateralUsd > 0, InvalidMinCollateral());
        minCollateralUsd = _minCollateralUsd;
        emit MinCollateralUpdated(_minCollateralUsd);
    }

    // ============ Operator Set Management ============

    function setOperatorSetId(uint32 _operatorSetId) external onlyOwner {
        require(_operatorSetId > 0, "Invalid operator set ID");
        require(
            operatorSetId == 0 || operatorSetId != _operatorSetId,
            "Already set to this ID"
        );

        uint32 previousId = operatorSetId;
        operatorSetId = _operatorSetId;

        emit OperatorSetIdUpdated(previousId, _operatorSetId);
    }

    function getAllocatedMagnitude(
        address operator,
        IStrategy strategy
    ) external view returns (uint256) {
        IAllocationManager.OperatorSet memory operatorSet = IAllocationManager
            .OperatorSet({ avs: address(this), operatorSetId: operatorSetId });

        return
            allocationManager.getAllocatedMagnitude(
                operator,
                operatorSet,
                strategy
            );
    }

    function getTotalMagnitude(
        address operator,
        IStrategy strategy
    ) external view returns (uint256) {
        return allocationManager.getTotalMagnitude(operator, strategy);
    }

    // ============ Operator Registration ============

    function registerOperatorToAVS(
        address operator,
        bytes calldata operatorSignature
    ) external onlyOwner {
        _avsDirectory.registerOperatorToAVS(operator, operatorSignature);

        emit OperatorRegisteredToAVS(operator);
    }

    function deregisterOperatorFromAVS(address operator) external onlyOwner {
        _avsDirectory.deregisterOperatorFromAVS(operator);

        emit OperatorDeregisteredFromAVS(operator);
    }

    function registerCiphernode(
        bytes calldata avsSignature
    ) external nonReentrant {
        require(!registeredOperators[msg.sender], OperatorNotRegistered());

        // 1. Verify operator is registered with EigenLayer DelegationManager
        require(
            _delegationManager.isOperator(msg.sender),
            OperatorNotRegistered()
        );

        // 2. Register operator to our AVS
        if (avsSignature.length > 0) {
            _avsDirectory.registerOperatorToAVS(msg.sender, avsSignature);
        }

        // 3. Verify operator has allocated sufficient magnitude to our operator set
        require(
            _verifyMagnitudeAllocation(msg.sender),
            InsufficientMagnitudeAllocation()
        );

        // 4. Check collateral requirements based on allocated stake
        (bool isEligible, uint256 collateralUsd) = checkOperatorEligibility(
            msg.sender
        );
        require(isEligible, InsufficientCollateral());

        // 5. Register with our system
        registeredOperators[msg.sender] = true;

        // 6. Register with bonding manager
        bondingManager.registerOperator(msg.sender, collateralUsd);

        // 7. Add to ciphernode registry
        ciphernodeRegistry.addCiphernode(msg.sender);

        emit CiphernodeRegistered(msg.sender, collateralUsd);
    }

    function deregisterCiphernode(
        uint256[] calldata siblingNodes
    ) external nonReentrant {
        require(registeredOperators[msg.sender], OperatorNotRegistered());

        registeredOperators[msg.sender] = false;
        bondingManager.deregisterOperator(msg.sender);
        ciphernodeRegistry.removeCiphernode(msg.sender, siblingNodes);

        emit CiphernodeDeregistered(msg.sender);
    }

    // ============ Slashing ============

    function addSlasher(address slasher) external onlyOwner {
        require(slasher != address(0), ZeroAddress());
        slashers[slasher] = true;
    }

    function removeSlasher(address slasher) external onlyOwner {
        slashers[slasher] = false;
    }

    function slashOperator(
        address operator,
        uint256 slashingPercentage,
        string calldata reason
    ) external nonReentrant {
        require(slashers[msg.sender], NotAuthorizedSlasher());
        require(registeredOperators[operator], OperatorNotRegistered());
        require(slashingPercentage <= 10000, InvalidSlashingPercentage());

        (
            IStrategy[] memory strategies,
            uint256[] memory wadsToSlash
        ) = _calculateSlashingWads(operator, slashingPercentage);

        if (strategies.length > 0) {
            // Create slashing parameters for EigenLayer AllocationManager
            IAllocationManager.SlashingParams
                memory slashingParams = IAllocationManager.SlashingParams({
                    operator: operator,
                    operatorSetId: operatorSetId,
                    strategies: strategies,
                    wadsToSlash: wadsToSlash,
                    description: reason
                });

            try
                allocationManager.slashOperator(address(this), slashingParams)
            returns (uint256 slashId, uint256[] memory slashedShares) {
                emit OperatorSlashed(
                    operator,
                    slashingPercentage,
                    reason,
                    strategies,
                    slashedShares
                );

                (
                    bool isEligible,
                    uint256 newCollateralUsd
                ) = checkOperatorEligibility(operator);
                if (!isEligible) {
                    // Auto-deregister if below minimum
                    registeredOperators[operator] = false;
                    bondingManager.deregisterOperator(operator);
                    emit CiphernodeDeregistered(operator);
                } else {
                    bondingManager.updateOperatorCollateral(
                        operator,
                        newCollateralUsd
                    );
                }
            } catch Error(string memory errorMsg) {
                revert(string(abi.encodePacked("Slashing failed: ", errorMsg)));
            }
        }
    }

    // ============ View Functions ============

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
        return strategyManager.stakerStrategyShares(operator, strategy);
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

    function getOperatorSetInfo() external view returns (uint32, address) {
        return (operatorSetId, address(this));
    }

    // ============ Internal Functions ============

    function _verifyMagnitudeAllocation(
        address operator
    ) internal view returns (bool) {
        IAllocationManager.OperatorSet memory operatorSet = IAllocationManager
            .OperatorSet({ avs: address(this), operatorSetId: operatorSetId });

        uint256 totalAllocatedCollateral = 0;

        for (uint256 i = 0; i < allowedStrategies.length; i++) {
            IStrategy strategy = allowedStrategies[i];

            uint256 totalMagnitude = allocationManager.getTotalMagnitude(
                operator,
                strategy
            );

            uint256 allocatedMagnitude = allocationManager
                .getAllocatedMagnitude(operator, operatorSet, strategy);

            if (totalMagnitude == 0) continue;
            uint256 totalShares = getOperatorShares(operator, strategy);

            if (totalShares > 0 && allocatedMagnitude > 0) {
                uint256 allocatedShares = (totalShares * allocatedMagnitude) /
                    totalMagnitude;

                uint256 allocatedUsd = _convertSharesToUsd(
                    strategy,
                    allocatedShares
                );
                totalAllocatedCollateral += allocatedUsd;
            }
        }

        return totalAllocatedCollateral >= minCollateralUsd;
    }

    function _convertSharesToUsd(
        IStrategy strategy,
        uint256 shares
    ) internal view returns (uint256 usdValue) {
        StrategyConfig memory config = strategyConfigs[strategy];

        uint256 underlyingAmount = strategy.sharesToUnderlyingView(shares);

        if (config.priceFeed == address(0)) {
            usdValue = underlyingAmount * (10 ** (18 - config.decimals));
        } else {
            uint256 price = _getTokenPrice(config.priceFeed);
            usdValue = (underlyingAmount * price) / (10 ** config.decimals);
        }
    }

    function _getTokenPrice(
        address priceFeed
    ) internal view returns (uint256 price) {
        AggregatorV3Interface feed = AggregatorV3Interface(priceFeed);

        try feed.latestRoundData() returns (
            uint80 /* roundId */,
            int256 answer,
            uint256 /* startedAt */,
            uint256 updatedAt,
            uint80 /* answeredInRound */
        ) {
            require(
                answer > 0 &&
                    block.timestamp - updatedAt <= PRICE_STALENESS_THRESHOLD,
                "Invalid or stale price"
            );
            return uint256(answer) * 1e10;
        } catch {
            revert("Price feed error");
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
        uint256 strategiesCount = 0;
        for (uint256 i = 0; i < allowedStrategies.length; i++) {
            if (getOperatorShares(operator, allowedStrategies[i]) > 0) {
                strategiesCount++;
            }
        }

        strategies = new IStrategy[](strategiesCount);
        wadsToSlash = new uint256[](strategiesCount);

        uint256 index = 0;
        uint256 slashingWad = (slashingPercentage * 1e18) / 10000;

        for (uint256 i = 0; i < allowedStrategies.length; i++) {
            IStrategy strategy = allowedStrategies[i];
            uint256 shares = getOperatorShares(operator, strategy);

            if (shares > 0) {
                strategies[index] = strategy;
                wadsToSlash[index] = slashingWad;
                index++;
            }
        }
    }
}
