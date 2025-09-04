// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {
    IStrategy
} from "eigenlayer-contracts/src/contracts/interfaces/IStrategy.sol";
import {
    IStrategyManager
} from "eigenlayer-contracts/src/contracts/interfaces/IStrategyManager.sol";
import {
    IAVSRegistrar
} from "eigenlayer-contracts/src/contracts/interfaces/IAVSRegistrar.sol";

/**
 * @title IServiceManager
 * @notice Interface for managing operator registration, rewards, and slashing
 */
interface IServiceManager {
    // ======================
    // General validation
    // ======================
    error ZeroAddress();
    error ArrayLengthMismatch();
    error InvalidPriceOrStale();
    error PriceFeedError();

    // ======================
    // Operator-related
    // ======================
    error OperatorNotRegistered();
    error OperatorNoStake();
    error InvalidOperatorSet();
    error InvalidOperatorSetId();
    error WrongOperatorSet();
    error AlreadySetToThisId();
    error NotLicensed();
    error MustDeregisterCiphernodeFirst();

    // ======================
    // Collateral & magnitude
    // ======================
    error InsufficientCollateral();
    error InvalidMinCollateral();
    error InsufficientAllocatedMagnitude();

    // ======================
    // Strategy-related
    // ======================
    error StrategyNotFound();
    error StrategyAlreadyAllowed();
    error NoStrategiesConfigured();
    error CannotRemoveCoreStrategies();

    // ======================
    // Slashing-related
    // ======================
    error InvalidWadSlashing();
    error InvalidAVS();
    error NotAuthorizedSlasher();
    error SlashingFailed(string reason);

    // ======================
    // Role/authorization
    // ======================
    error OnlyAllocationManager();
    error OnlyRewardDistributor();

    // Events
    event StrategyAdded(address indexed strategy, address priceFeed);
    event StrategyRemoved(address indexed strategy);
    event StrategyUpdated(address indexed strategy, address newPriceFeed);
    event MinCollateralUpdated(uint256 newMinCollateral);
    event OperatorSetIdUpdated(uint32 previousId, uint32 newId);
    event AVSRegistrarSet(address indexed registrar);
    event OperatorRegisteredToAVS(address indexed operator);
    event OperatorDeregisteredFromAVS(address indexed operator);
    event OperatorBonded(address indexed operator, uint256 collateralUsd);
    event OperatorDebonded(address indexed operator);
    event OperatorSlashed(
        address indexed operator,
        uint256 wadToSlash,
        string description,
        IStrategy[] strategies,
        uint256[] slashedShares
    );
    event SlasherAdded(address indexed slasher);
    event SlasherRemoved(address indexed slasher);
    event BondingManagerSet(address indexed bondingManager);

    // Structs
    struct StrategyConfig {
        bool isAllowed;
        address priceFeed;
        uint8 decimals;
    }

    struct OperatorInfo {
        bool isActive;
        uint256 registeredAt;
        uint256 collateralUsd;
    }

    /**
     * @notice Add strategy to allowed strategies list
     * @param strategy Strategy contract address
     * @param priceFeed Chainlink price feed address (0x0 for stablecoins)
     */
    function addStrategy(IStrategy strategy, address priceFeed) external;

    /**
     * @notice Remove strategy from allowed strategies list
     * @param strategy Strategy contract address
     */
    function removeStrategy(IStrategy strategy) external;

    /**
     * @notice Update strategy price feed
     * @param strategy Strategy contract address
     * @param newPriceFeed New price feed address
     */
    function updateStrategy(IStrategy strategy, address newPriceFeed) external;

    /**
     * @notice Set minimum collateral USD requirement
     * @param _minCollateralUsd New minimum collateral in USD
     */
    function setMinCollateralUsd(uint256 _minCollateralUsd) external;

    /**
     * @notice Set operator set ID for this AVS
     * @param _operatorSetId New operator set ID
     */
    function setOperatorSetId(uint32 _operatorSetId) external;

    /**
     * @notice Set AVS registrar in AllocationManager
     * @param registrar New registrar contract
     */
    function setAVSRegistrar(IAVSRegistrar registrar) external;

    /**
     * @notice Set reward distributor address
     * @param _rewardDistributor New reward distributor address
     */
    function setRewardDistributor(address _rewardDistributor) external;

    /**
     * @notice Set bonding manager address
     * @param _bondingManager New bonding manager address
     */
    function setBondingManager(address _bondingManager) external;

    /**
     * @notice Publish AVS metadata URI via AllocationManager
     * @param uri Metadata URI for the AVS
     */
    function publishAVSMetadata(string calldata uri) external;

    /**
     * @notice Create operator set with strategies via AllocationManager
     * @param id Operator set ID
     * @param strategies Array of strategy addresses
     */
    function createOperatorSet(
        uint32 id,
        IStrategy[] calldata strategies
    ) external;

    /**
     * @notice Add strategies to existing operator set via AllocationManager
     * @param id Operator set ID
     * @param strategies Array of strategy addresses to add
     */
    function addStrategies(uint32 id, IStrategy[] calldata strategies) external;

    /**
     * @notice Add authorized slasher
     * @param slasher Address to authorize for slashing
     */
    function addSlasher(address slasher) external;

    /**
     * @notice Remove authorized slasher
     * @param slasher Address to remove from slashing authorization
     */
    function removeSlasher(address slasher) external;

    /**
     * @notice Slash operator across all strategies
     * @param operator Address of the operator to slash
     * @param wadToSlash Proportion to slash (in WAD format, 1e18 = 100%)
     * @param description Description of the slashing reason
     */
    function slashOperator(
        address operator,
        uint256 wadToSlash,
        string calldata description
    ) external;

    /**
     * @notice Distribute rewards to operators
     * @param recipients Array of operator addresses
     * @param amounts Array of reward amounts
     */
    function distributeRewards(
        address[] calldata recipients,
        uint256[] calldata amounts
    ) external;

    /**
     * @notice Check if operator meets collateral requirements
     * @param operator Address of the operator
     * @return isEligible True if operator meets requirements
     * @return collateralUsd USD value of operator's collateral
     */
    function checkOperatorEligibility(
        address operator
    ) external view returns (bool isEligible, uint256 collateralUsd);

    /**
     * @notice Get operator's collateral value in USD
     * @param operator Address of the operator
     * @return totalUsdValue Total USD value of operator's collateral
     */
    function getOperatorCollateralValue(
        address operator
    ) external view returns (uint256 totalUsdValue);

    /**
     * @notice Get operator's shares in a strategy
     * @param operator Address of the operator
     * @param strategy Strategy contract address
     * @return shares Number of shares
     */
    function getOperatorShares(
        address operator,
        IStrategy strategy
    ) external view returns (uint256 shares);

    /**
     * @notice Get operator's allocated magnitude for a strategy
     * @param operator Address of the operator
     * @param strategy Strategy contract address
     * @return Allocated magnitude
     */
    function getAllocatedMagnitude(
        address operator,
        IStrategy strategy
    ) external view returns (uint256);

    /**
     * @notice Get operator's total magnitude for a strategy
     * @param operator Address of the operator
     * @param strategy Strategy contract address
     * @return Total magnitude (allocated + allocatable)
     */
    function getTotalMagnitude(
        address operator,
        IStrategy strategy
    ) external view returns (uint256);

    /**
     * @notice Check if strategy is allowed
     * @param strategy Strategy contract address
     * @return True if strategy is allowed
     */
    function isStrategyAllowed(IStrategy strategy) external view returns (bool);

    /**
     * @notice Get all allowed strategies
     * @return Array of allowed strategy addresses
     */
    function getAllowedStrategies() external view returns (IStrategy[] memory);

    /**
     * @notice Get strategy configuration
     * @param strategy Strategy contract address
     * @return Price feed address
     */
    function getStrategyConfig(
        IStrategy strategy
    ) external view returns (address);

    /**
     * @notice Get minimum collateral USD requirement
     * @return Minimum collateral in USD
     */
    function getMinCollateralUsd() external view returns (uint256);

    /**
     * @notice Get the strategy manager contract
     * @return The IStrategyManager instance
     */
    function strategyManager() external view returns (IStrategyManager);

    /**
     * @notice Get operator set information
     * @return operatorSetId The operator set ID
     * @return avs The AVS address
     */
    function getOperatorSetInfo() external view returns (uint32, address);

    /**
     * @notice Get operator information
     * @param operator Address of the operator
     * @return OperatorInfo struct
     */
    function getOperatorInfo(
        address operator
    ) external view returns (OperatorInfo memory);
}
