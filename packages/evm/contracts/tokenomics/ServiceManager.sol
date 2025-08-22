// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import {
    ServiceManagerBase
} from "@eigenlayer-middleware/src/ServiceManagerBase.sol";

import {
    IStakeRegistry
} from "@eigenlayer-middleware/src/interfaces/IStakeRegistry.sol";
import {
    IRewardsCoordinator
} from "eigenlayer-contracts/src/contracts/interfaces/IRewardsCoordinator.sol";
import {
    IPermissionController
} from "eigenlayer-contracts/src/contracts/interfaces/IPermissionController.sol";
import {
    ISlashingRegistryCoordinator
} from "@eigenlayer-middleware/src/interfaces/ISlashingRegistryCoordinator.sol";
import {
    ISignatureUtilsMixinTypes
} from "eigenlayer-contracts/src/contracts/interfaces/ISignatureUtilsMixin.sol";
import {
    IStrategy
} from "eigenlayer-contracts/src/contracts/interfaces/IStrategy.sol";
import {
    IAllocationManager,
    IAllocationManagerTypes
} from "eigenlayer-contracts/src/contracts/interfaces/IAllocationManager.sol";
import {
    IAVSRegistrar
} from "eigenlayer-contracts/src/contracts/interfaces/IAVSRegistrar.sol";
import {
    IDelegationManager
} from "eigenlayer-contracts/src/contracts/interfaces/IDelegationManager.sol";
import {
    IAVSDirectory
} from "eigenlayer-contracts/src/contracts/interfaces/IAVSDirectory.sol";
import {
    IStrategyManager
} from "eigenlayer-contracts/src/contracts/interfaces/IStrategyManager.sol";
import {
    OperatorSetLib,
    OperatorSet
} from "eigenlayer-contracts/src/contracts/libraries/OperatorSetLib.sol";
import {
    AggregatorV3Interface
} from "@chainlink/contracts/src/v0.8/shared/interfaces/AggregatorV3Interface.sol";

import { ReentrancyGuard } from "@oz/utils/ReentrancyGuard.sol";
import { IERC20Metadata } from "@oz/token/ERC20/extensions/IERC20Metadata.sol";
import { SafeERC20 } from "@oz/token/ERC20/utils/SafeERC20.sol";
import { Math } from "@oz/utils/math/Math.sol";

import { IServiceManager } from "../interfaces/IServiceManager.sol";
import { ICiphernodeRegistry } from "../interfaces/ICiphernodeRegistry.sol";
import { IBondingManager } from "../interfaces/IBondingManager.sol";

contract ServiceManager is
    ServiceManagerBase,
    IServiceManager,
    ReentrancyGuard
{
    using SafeERC20 for IERC20Metadata;

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

    /// @notice EigenLayer StrategyManager for share queries
    IStrategyManager public strategyManager;

    /// @notice EigenLayer DelegationManager for operator registration
    IDelegationManager public delegationManager;

    /// @notice Addresses authorized to slash operators
    mapping(address slasher => bool isAuthorized) public slashers;

    /// @notice Registered operators
    mapping(address operator => bool isRegistered) public registeredOperators;

    /// @notice Operator set ID
    uint32 public operatorSetId;

    // ============ Dual Bonding System Variables ============

    /// @notice ENCL token strategy for license staking
    IStrategy public enclStrategy;

    /// @notice USDC token strategy for ticket purchases
    IStrategy public usdcStrategy;

    /// @notice Amount of ENCL required to acquire license (18 decimals)
    uint256 public licenseStake;

    /// @notice Price per selection ticket in USDC (6 decimals)
    uint256 public ticketPrice;

    /// @notice Operator information mapping
    mapping(address operator => OperatorInfo info) public operatorInfos;

    /// @notice Reward distributor address
    address public rewardDistributor;

    /// @notice Track USDC spent on tickets per operator
    mapping(address operator => uint256 spent) public ticketBudgetSpent;

    constructor(
        IAVSDirectory _avsDirectory,
        IRewardsCoordinator _rewardsCoordinator,
        ISlashingRegistryCoordinator _registryCoordinator,
        IStakeRegistry _stakeRegistry,
        IPermissionController _permissionController,
        IAllocationManager _allocationManager
    )
        ServiceManagerBase(
            _avsDirectory,
            _rewardsCoordinator,
            _registryCoordinator,
            _stakeRegistry,
            _permissionController,
            _allocationManager
        )
    {
        _disableInitializers();
    }

    /**
     * @notice Initialize the ServiceManager with dual bonding configuration
     * @param _owner Owner of the contract
     * @param _rewardsInitiator Address that can initiate rewards
     * @param _strategyManager EigenLayer StrategyManager
     * @param _delegationManager EigenLayer DelegationManager
     * @param _ciphernodeRegistry Ciphernode registry contract
     * @param _bondingManager Bonding manager contract
     * @param _enclStrategy Strategy for ENCL token
     * @param _usdcStrategy Strategy for USDC token
     * @param _licenseStake Required ENCL stake for license
     * @param _ticketPrice Price per ticket in USDC
     * @param _minCollateralUsd Minimum collateral in USD
     * @param _operatorSetId Operator set ID
     */
    function initialize(
        address _owner,
        address _rewardsInitiator,
        IStrategyManager _strategyManager,
        IDelegationManager _delegationManager,
        address _ciphernodeRegistry,
        address _bondingManager,
        IStrategy _enclStrategy,
        IStrategy _usdcStrategy,
        uint256 _licenseStake,
        uint256 _ticketPrice,
        uint256 _minCollateralUsd,
        uint32 _operatorSetId
    ) external reinitializer(1) {
        require(_ciphernodeRegistry != address(0), ZeroAddress());
        require(_bondingManager != address(0), ZeroAddress());
        require(address(_strategyManager) != address(0), ZeroAddress());
        require(address(_delegationManager) != address(0), ZeroAddress());
        require(address(_enclStrategy) != address(0), ZeroAddress());
        require(address(_usdcStrategy) != address(0), ZeroAddress());
        require(_licenseStake > 0, InvalidMinCollateral());
        require(_ticketPrice > 0, InvalidTicketAmount());
        require(_minCollateralUsd > 0, InvalidMinCollateral());
        require(_operatorSetId > 0, "Invalid operator set ID");

        // Initialize ServiceManagerBase
        __ServiceManagerBase_init(_owner, _rewardsInitiator);

        // Set contract addresses
        strategyManager = _strategyManager;
        delegationManager = _delegationManager;
        ciphernodeRegistry = ICiphernodeRegistry(_ciphernodeRegistry);
        bondingManager = IBondingManager(_bondingManager);

        // Dual bonding parameters
        enclStrategy = _enclStrategy;
        usdcStrategy = _usdcStrategy;
        licenseStake = _licenseStake;
        ticketPrice = _ticketPrice;
        minCollateralUsd = _minCollateralUsd;
        operatorSetId = _operatorSetId;

        // Default strategies
        _addStrategyInternal(_enclStrategy, address(0));
        _addStrategyInternal(_usdcStrategy, address(0));
    }

    // ============ Dual Bonding System - License Management ============

    /**
     * @notice Acquire license by staking required ENCL tokens
     * @dev Operator must be an EigenLayer operator and stake ENCL
     */
    function acquireLicense() external nonReentrant {
        require(
            delegationManager.isOperator(msg.sender),
            OperatorNotRegistered()
        );
        require(!operatorInfos[msg.sender].isLicensed, AlreadyLicensed());

        uint256 enclShares = getOperatorShares(msg.sender, enclStrategy);
        uint256 enclAmount = enclStrategy.sharesToUnderlyingView(enclShares);

        require(enclAmount >= licenseStake, InsufficientLicenseStake());

        operatorInfos[msg.sender] = OperatorInfo({
            isLicensed: true,
            licenseStake: enclAmount,
            ticketBalance: 0,
            registeredAt: block.timestamp
        });

        emit LicenseAcquired(msg.sender, enclAmount);
    }

    /**
     * @notice Purchase selection tickets with USDC
     * @param ticketCount Number of tickets to purchase
     */
    function purchaseTickets(uint256 ticketCount) external nonReentrant {
        require(ticketCount > 0, InvalidTicketAmount());
        require(operatorInfos[msg.sender].isLicensed, NotLicensed());

        uint256 totalCost = ticketCount * ticketPrice;
        require(totalCost / ticketPrice == ticketCount, "Cost overflow");

        uint256 usdcShares = getOperatorShares(msg.sender, usdcStrategy);
        uint256 usdcAmount = usdcStrategy.sharesToUnderlyingView(usdcShares);
        uint256 alreadySpent = ticketBudgetSpent[msg.sender];
        uint256 availableBudget = usdcAmount > alreadySpent
            ? usdcAmount - alreadySpent
            : 0;

        require(availableBudget >= totalCost, "Insufficient ticket budget");

        ticketBudgetSpent[msg.sender] = alreadySpent + totalCost;
        operatorInfos[msg.sender].ticketBalance += ticketCount;

        emit TicketsPurchased(msg.sender, totalCost, ticketCount);
    }

    // ============ Strategy Management ============

    function addStrategy(
        IStrategy strategy,
        address priceFeed
    ) external onlyOwner {
        _addStrategyInternal(strategy, priceFeed);
    }

    function _addStrategyInternal(
        IStrategy strategy,
        address priceFeed
    ) internal {
        require(address(strategy) != address(0), ZeroAddress());
        require(!strategyConfigs[strategy].isAllowed, StrategyAlreadyAllowed());

        uint8 decimals = 18;
        try
            IERC20Metadata(address(strategy.underlyingToken())).decimals()
        returns (uint8 d) {
            decimals = d;
        } catch {
            // Defaults to 18 decimals
        }

        strategyConfigs[strategy] = StrategyConfig({
            isAllowed: true,
            priceFeed: priceFeed,
            decimals: decimals
        });

        strategyToIndex[strategy] = allowedStrategies.length;
        allowedStrategies.push(strategy);

        emit StrategyAdded(address(strategy), priceFeed);
    }

    function removeStrategy(IStrategy strategy) external onlyOwner {
        require(strategyConfigs[strategy].isAllowed, StrategyNotFound());
        require(
            strategy != enclStrategy && strategy != usdcStrategy,
            "Cannot remove core strategies"
        );

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

        emit StrategyRemoved(address(strategy));
    }

    function updateStrategy(
        IStrategy strategy,
        address newPriceFeed
    ) external onlyOwner {
        require(strategyConfigs[strategy].isAllowed, StrategyNotFound());

        strategyConfigs[strategy].priceFeed = newPriceFeed;

        emit StrategyUpdated(address(strategy), newPriceFeed);
    }

    function setMinCollateralUsd(uint256 _minCollateralUsd) external onlyOwner {
        require(_minCollateralUsd > 0, InvalidMinCollateral());
        minCollateralUsd = _minCollateralUsd;
        emit MinCollateralUpdated(_minCollateralUsd);
    }

    function setLicenseStake(uint256 _licenseStake) external onlyOwner {
        require(_licenseStake > 0, InvalidMinCollateral());
        licenseStake = _licenseStake;
        emit LicenseStakeUpdated(_licenseStake);
    }

    function setTicketPrice(uint256 _ticketPrice) external onlyOwner {
        require(_ticketPrice > 0, InvalidTicketAmount());
        ticketPrice = _ticketPrice;
        emit TicketPriceUpdated(_ticketPrice);
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

    function setAVSRegistrar(IAVSRegistrar registrar) external onlyOwner {
        _allocationManager.setAVSRegistrar(address(this), registrar);
        emit AVSRegistrarSet(address(registrar));
    }

    function getAllocatedMagnitude(
        address operator,
        IStrategy strategy
    ) external view returns (uint256) {
        OperatorSet memory operatorSet = OperatorSet({
            avs: address(this),
            id: operatorSetId
        });

        return
            uint256(
                _allocationManager
                    .getAllocation(operator, operatorSet, strategy)
                    .currentMagnitude
            );
    }

    function getTotalMagnitude(
        address operator,
        IStrategy strategy
    ) external view returns (uint256) {
        return
            uint256(
                _allocationManager.getEncumberedMagnitude(operator, strategy)
            ) +
            uint256(
                _allocationManager.getAllocatableMagnitude(operator, strategy)
            );
    }

    // ============ Operator Registration ============

    function registerOperatorToAVS(
        address operator,
        ISignatureUtilsMixinTypes.SignatureWithSaltAndExpiry
            memory operatorSignature
    ) public override onlyRegistryCoordinator {
        _avsDirectory.registerOperatorToAVS(operator, operatorSignature);
        emit OperatorRegisteredToAVS(operator);
    }

    function deregisterOperatorFromAVS(
        address operator
    ) public override onlyRegistryCoordinator {
        _avsDirectory.deregisterOperatorFromAVS(operator);
        emit OperatorDeregisteredFromAVS(operator);
    }

    function deregisterOperatorFromOperatorSets(
        address operator,
        uint32[] memory operatorSetIds
    )
        public
        override(ServiceManagerBase, IServiceManager)
        onlyRegistryCoordinator
    {
        super.deregisterOperatorFromOperatorSets(operator, operatorSetIds);

        for (uint256 i = 0; i < operatorSetIds.length; i++) {
            if (operatorSetIds[i] == operatorSetId) {
                registeredOperators[operator] = false;
                operatorInfos[operator].isLicensed = false;
            }
        }
    }

    function registerCiphernode() external nonReentrant {
        require(
            !registeredOperators[msg.sender],
            "Already registered as ciphernode"
        );
        require(operatorInfos[msg.sender].isLicensed, NotLicensed());

        // 1. Verify operator registration
        require(
            delegationManager.isOperator(msg.sender),
            OperatorNotRegistered()
        );

        // 2. Check license stake is still sufficient
        uint256 enclShares = getOperatorShares(msg.sender, enclStrategy);
        uint256 enclAmount = enclStrategy.sharesToUnderlyingView(enclShares);
        require(enclAmount >= licenseStake, InsufficientLicenseStake());

        // 3. Check collateral requirements
        (bool isEligible, uint256 collateralUsd) = checkOperatorEligibility(
            msg.sender
        );
        require(isEligible, InsufficientCollateral());

        registeredOperators[msg.sender] = true;
        bondingManager.registerOperator(msg.sender, collateralUsd);
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
        emit SlasherAdded(slasher);
    }

    function removeSlasher(address slasher) external onlyOwner {
        slashers[slasher] = false;
        emit SlasherRemoved(slasher);
    }

    function slashOperator(
        address operator,
        uint256 wadToSlash,
        string calldata description
    ) external nonReentrant {
        require(slashers[msg.sender], NotAuthorizedSlasher());
        require(registeredOperators[operator], OperatorNotRegistered());
        require(wadToSlash <= 1e18 && wadToSlash > 0, "wad out of range");
        require(allowedStrategies.length > 0, "No strategies configured");

        // Filter to strategies where operator has shares
        IStrategy[] memory tmpStrategies = new IStrategy[](
            allowedStrategies.length
        );
        uint256 count = 0;

        for (uint256 i = 0; i < allowedStrategies.length; i++) {
            if (getOperatorShares(operator, allowedStrategies[i]) > 0) {
                tmpStrategies[count] = allowedStrategies[i];
                count++;
            }
        }

        require(count > 0, "Operator has no stake in any strategy");

        IStrategy[] memory strategies = new IStrategy[](count);
        uint256[] memory wadsToSlash = new uint256[](count);

        for (uint256 i = 0; i < count; i++) {
            strategies[i] = tmpStrategies[i];
            wadsToSlash[i] = wadToSlash;
        }

        IAllocationManagerTypes.SlashingParams
            memory params = IAllocationManagerTypes.SlashingParams({
                operator: operator,
                operatorSetId: operatorSetId,
                strategies: strategies,
                wadsToSlash: wadsToSlash,
                description: description
            });

        try _allocationManager.slashOperator(address(this), params) returns (
            uint256 /* slashId */,
            uint256[] memory slashedShares
        ) {
            emit OperatorSlashed(
                operator,
                wadToSlash,
                description,
                strategies,
                slashedShares
            );

            _checkAndUpdateLicenseStatus(operator);

            (
                bool isEligible,
                uint256 newCollateralUsd
            ) = checkOperatorEligibility(operator);
            if (isEligible) {
                bondingManager.updateOperatorCollateral(
                    operator,
                    newCollateralUsd
                );
            }
        } catch Error(string memory errorMsg) {
            revert(string(abi.encodePacked("Slashing failed: ", errorMsg)));
        }
    }

    // ============ Rewards ============

    function setRewardDistributor(
        address _rewardDistributor
    ) external onlyOwner {
        require(_rewardDistributor != address(0), ZeroAddress());
        rewardDistributor = _rewardDistributor;
    }

    function distributeRewards(
        address[] calldata recipients,
        uint256[] calldata amounts
    ) external {
        require(msg.sender == rewardDistributor, "Only reward distributor");
        require(recipients.length == amounts.length, "Array length mismatch");

        IERC20Metadata enclToken = IERC20Metadata(
            address(enclStrategy.underlyingToken())
        );

        for (uint256 i = 0; i < recipients.length; i++) {
            if (amounts[i] > 0 && registeredOperators[recipients[i]]) {
                enclToken.safeTransfer(recipients[i], amounts[i]);
            }
        }
    }

    function useTickets(address operator, uint256 ticketCount) external {
        require(msg.sender == address(ciphernodeRegistry), "Only registry");
        require(
            operatorInfos[operator].ticketBalance >= ticketCount,
            InsufficientTicketBalance()
        );

        operatorInfos[operator].ticketBalance -= ticketCount;
        emit TicketsUsed(operator, ticketCount);
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

            // Skip ENCL strategy - license tokens don't count as collateral
            if (strategy == enclStrategy) {
                continue;
            }

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
        // Use delegated shares (operator stake) instead of self-deposits
        IStrategy[] memory strategies = new IStrategy[](1);
        strategies[0] = strategy;
        uint256[] memory operatorShares = delegationManager.getOperatorShares(
            operator,
            strategies
        );
        return operatorShares[0];
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
    ) external view returns (address priceFeed) {
        StrategyConfig memory config = strategyConfigs[strategy];
        return config.priceFeed;
    }

    function getMinCollateralUsd() external view returns (uint256) {
        return minCollateralUsd;
    }

    function getOperatorSetInfo() external view returns (uint32, address) {
        return (operatorSetId, address(this));
    }

    function getOperatorInfo(
        address operator
    ) external view returns (OperatorInfo memory info) {
        return operatorInfos[operator];
    }

    function getLicenseStake() external view returns (uint256) {
        return licenseStake;
    }

    function getTicketPrice() external view returns (uint256) {
        return ticketPrice;
    }

    function getAvailableTicketBudget(
        address operator
    ) external view returns (uint256) {
        uint256 usdcShares = getOperatorShares(operator, usdcStrategy);
        uint256 usdcAmount = usdcStrategy.sharesToUnderlyingView(usdcShares);
        uint256 alreadySpent = ticketBudgetSpent[operator];
        return usdcAmount > alreadySpent ? usdcAmount - alreadySpent : 0;
    }

    // ============ Internal Functions ============

    function _checkAndUpdateLicenseStatus(address operator) internal {
        if (!operatorInfos[operator].isLicensed) return;

        uint256 enclShares = getOperatorShares(operator, enclStrategy);
        uint256 enclAmount = enclStrategy.sharesToUnderlyingView(enclShares);

        if (enclAmount < licenseStake) {
            // Revoke license if below threshold
            operatorInfos[operator].isLicensed = false;

            // Auto-deregister from ciphernode registry
            if (registeredOperators[operator]) {
                registeredOperators[operator] = false;
                bondingManager.deregisterOperator(operator);
                // Note: Cannot auto-remove from registry without sibling nodes
                emit CiphernodeDeregistered(operator);
            }

            emit LicenseRevoked(operator);
        } else if (enclAmount < (operatorInfos[operator].licenseStake / 2)) {
            // Initiate decommission process
            operatorInfos[operator].isLicensed = false;
            if (registeredOperators[operator]) {
                registeredOperators[operator] = false;
                bondingManager.deregisterOperator(operator);
                emit CiphernodeDeregistered(operator);
            }
            emit LicenseRevoked(operator);
        }
    }

    function _convertSharesToUsd(
        IStrategy strategy,
        uint256 shares
    ) internal view returns (uint256 usdValue) {
        StrategyConfig memory config = strategyConfigs[strategy];

        uint256 underlyingAmount = strategy.sharesToUnderlyingView(shares);

        if (config.priceFeed == address(0)) {
            usdValue = Math.mulDiv(
                underlyingAmount,
                1e18,
                10 ** config.decimals
            );
        } else {
            uint256 price = _getTokenPrice(config.priceFeed);

            usdValue = Math.mulDiv(
                underlyingAmount,
                price,
                10 ** config.decimals
            );
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

            uint8 feedDecimals = feed.decimals();
            uint256 rawPrice = uint256(answer);

            if (feedDecimals < 18) {
                return rawPrice * (10 ** (18 - feedDecimals));
            } else if (feedDecimals > 18) {
                return rawPrice / (10 ** (feedDecimals - 18));
            } else {
                return rawPrice;
            }
        } catch {
            revert("Price feed error");
        }
    }
}
