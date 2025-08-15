// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import {
    IStrategy
} from "../../lib/eigenlayer-contracts/src/contracts/interfaces/IStrategy.sol";

interface IServiceManager {
    /// @notice Custom errors
    error ZeroAddress();
    error StrategyNotAllowed();
    error InsufficientCollateral();
    error OperatorNotRegistered();
    error NotAuthorizedSlasher();
    error InvalidMinCollateral();
    error StrategyAlreadyAllowed();
    error StrategyNotFound();
    error InvalidSlashingPercentage();

    /// @notice Events
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
    event CiphernodeRegistered(address indexed operator, uint256 collateralUsd);
    event CiphernodeDeregistered(address indexed operator);
    event OperatorSlashed(
        address indexed operator,
        uint256 slashingPercentage,
        string reason,
        IStrategy[] strategies,
        uint256[] slashedShares
    );
    event SlashingOperatorSetReady(
        uint32 indexed operatorSetId,
        IStrategy[] strategies
    );

    /// @notice Strategy configuration
    struct StrategyConfig {
        bool isAllowed;
        uint256 minShares;
        address priceFeed;
        uint8 decimals;
    }

    /**
     * @notice Add a supported strategy for collateral
     * @param strategy The EigenLayer strategy contract
     * @param minShares Minimum shares required in this strategy
     * @param priceFeed Chainlink price feed for USD conversion (address(0) for stablecoins)
     */
    function addStrategy(
        IStrategy strategy,
        uint256 minShares,
        address priceFeed
    ) external;

    /**
     * @notice Remove a supported strategy
     * @param strategy The strategy to remove
     */
    function removeStrategy(IStrategy strategy) external;

    /**
     * @notice Update strategy parameters
     * @param strategy The strategy to update
     * @param newMinShares New minimum shares requirement
     * @param newPriceFeed New price feed address
     */
    function updateStrategy(
        IStrategy strategy,
        uint256 newMinShares,
        address newPriceFeed
    ) external;

    /**
     * @notice Set minimum collateral requirement in USD
     * @param minCollateralUsd New minimum collateral in USD (18 decimals)
     */
    function setMinCollateralUsd(uint256 minCollateralUsd) external;

    /**
     * @notice Register as a ciphernode (permissionless)
     * @dev Operator must be registered with EigenLayer and have sufficient restaked collateral
     */
    function registerCiphernode() external;

    /**
     * @notice Deregister from being a ciphernode
     * @param siblingNodes Array of sibling node indices for registry removal
     */
    function deregisterCiphernode(uint256[] calldata siblingNodes) external;

    /**
     * @notice Ensure slashing operator set is ready for EigenLayer integration
     * @dev Creates minimal operator set for slashing without requiring operator allocation management
     */
    function ensureSlashingOperatorSet() external;

    /**
     * @notice Slash an operator's collateral for misbehavior
     * @param operator Address of the operator to slash
     * @param slashingPercentage Percentage to slash in basis points (e.g., 500 = 5%)
     * @param reason Reason for slashing
     * @dev Only authorized slashers can call this.
     */
    function slashOperator(
        address operator,
        uint256 slashingPercentage,
        string calldata reason
    ) external;

    /**
     * @notice Check if an operator meets collateral requirements
     * @param operator Address of the operator
     * @return isEligible Whether the operator meets requirements
     * @return collateralUsd Total collateral value in USD (18 decimals)
     */
    function checkOperatorEligibility(
        address operator
    ) external view returns (bool isEligible, uint256 collateralUsd);

    /**
     * @notice Get total collateral value for an operator across all strategies
     * @param operator Address of the operator
     * @return totalUsdValue Total USD value of operator's restaked collateral (18 decimals)
     */
    function getOperatorCollateralValue(
        address operator
    ) external view returns (uint256 totalUsdValue);

    /**
     * @notice Get operator's shares in a specific strategy
     * @param operator Address of the operator
     * @param strategy The strategy contract
     * @return shares Amount of shares the operator has in the strategy
     */
    function getOperatorShares(
        address operator,
        IStrategy strategy
    ) external view returns (uint256 shares);

    /**
     * @notice Check if a strategy is allowed for collateral
     * @param strategy The strategy to check
     * @return isAllowed Whether the strategy is allowed
     */
    function isStrategyAllowed(
        IStrategy strategy
    ) external view returns (bool isAllowed);

    /**
     * @notice Get all allowed strategies
     * @return strategies Array of allowed strategy contracts
     */
    function getAllowedStrategies()
        external
        view
        returns (IStrategy[] memory strategies);

    /**
     * @notice Get strategy configuration
     * @param strategy The strategy contract
     * @return minShares Minimum shares required
     * @return priceFeed Price feed address for USD conversion
     */
    function getStrategyConfig(
        IStrategy strategy
    ) external view returns (uint256 minShares, address priceFeed);

    /**
     * @notice Get minimum collateral requirement
     * @return minCollateralUsd Minimum collateral in USD (18 decimals)
     */
    function getMinCollateralUsd()
        external
        view
        returns (uint256 minCollateralUsd);
}
