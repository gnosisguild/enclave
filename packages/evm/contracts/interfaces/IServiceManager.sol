// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import {
    IStrategy
} from "eigenlayer-contracts/src/contracts/interfaces/IStrategy.sol";

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
    error InsufficientMagnitudeAllocation();
    error InsufficientLicenseStake();
    error InsufficientTicketBalance();
    error InvalidTicketAmount();
    error TicketPurchaseFailed();
    error AlreadyLicensed();
    error NotLicensed();

    /// @notice Events
    event MinCollateralUpdated(uint256 newMinCollateralUsd);
    event CiphernodeRegistered(address indexed operator, uint256 collateralUsd);
    event CiphernodeDeregistered(address indexed operator);
    event OperatorSlashed(
        address indexed operator,
        uint256 wadToSlash,
        string description,
        IStrategy[] strategies,
        uint256[] slashedShares
    );
    event OperatorSetIdUpdated(
        uint32 indexed previousOperatorSetId,
        uint32 indexed newOperatorSetId
    );
    event OperatorRegisteredToAVS(address indexed operator);
    event OperatorDeregisteredFromAVS(address indexed operator);
    event AVSRegistrarSet(address indexed registrar);
    event SlasherAdded(address indexed slasher);
    event SlasherRemoved(address indexed slasher);
    event StrategyAdded(address indexed strategy, address indexed priceFeed);
    event StrategyRemoved(address indexed strategy);
    event StrategyUpdated(
        address indexed strategy,
        address indexed newPriceFeed
    );

    event LicenseAcquired(address indexed operator, uint256 enclAmount);
    event LicenseRevoked(address indexed operator);
    event TicketsPurchased(
        address indexed operator,
        uint256 usdcAmount,
        uint256 ticketCount
    );
    event TicketsUsed(address indexed operator, uint256 ticketCount);
    event LicenseStakeUpdated(uint256 newLicenseStake);
    event TicketPriceUpdated(uint256 newTicketPrice);

    /// @notice Strategy configuration
    struct StrategyConfig {
        bool isAllowed;
        address priceFeed;
        uint8 decimals;
    }

    /// @notice Operator license and ticket information
    struct OperatorInfo {
        bool isLicensed; // Has operator acquired license with ENCL stake?
        uint256 licenseStake; // Amount of ENCL staked for license
        uint256 ticketBalance; // Number of selection tickets owned
        uint256 registeredAt; // Timestamp when license was acquired
    }

    /**
     * @notice Add a supported strategy for collateral
     * @param strategy The EigenLayer strategy contract
     * @param priceFeed Chainlink price feed for USD conversion (address(0) for stablecoins)
     */
    function addStrategy(IStrategy strategy, address priceFeed) external;

    /**
     * @notice Remove a supported strategy
     * @param strategy The strategy to remove
     */
    function removeStrategy(IStrategy strategy) external;

    /**
     * @notice Update strategy parameters
     * @param strategy The strategy to update
     * @param newPriceFeed New price feed address
     */
    function updateStrategy(IStrategy strategy, address newPriceFeed) external;

    /**
     * @notice Set minimum collateral requirement in USD
     * @param minCollateralUsd New minimum collateral in USD (18 decimals)
     */
    function setMinCollateralUsd(uint256 minCollateralUsd) external;

    /**
     * @notice Set minimum ENCL stake required for license
     * @param licenseStake Amount of ENCL required for license
     */
    function setLicenseStake(uint256 licenseStake) external;

    /**
     * @notice Set price per selection ticket in USDC
     * @param ticketPrice Price per ticket in USDC (6 decimals)
     */
    function setTicketPrice(uint256 ticketPrice) external;

    /**
     * @notice Acquire license to become a ciphernode by staking ENCL
     * @dev Operator must first stake ENCL tokens to get license
     */
    function acquireLicense() external;

    /**
     * @notice Purchase selection tickets with USDC
     * @param ticketCount Number of tickets to purchase
     */
    function purchaseTickets(uint256 ticketCount) external;

    /**
     * @notice Register as a ciphernode (requires license)
     * @dev Operator must have license and meet collateral requirements
     */
    function registerCiphernode() external;

    /**
     * @notice Deregister from being a ciphernode
     * @param siblingNodes Array of sibling node indices for registry removal
     */
    function deregisterCiphernode(uint256[] calldata siblingNodes) external;

    /**
     * @notice Slash an operator's collateral for misbehavior
     * @param operator Address of the operator to slash
     * @param wadToSlash Amount to slash in WAD format (18 decimals, 1e18 = 100%)
     * @param description Description of the slashing event
     * @dev Only authorized slashers can call this
     */
    function slashOperator(
        address operator,
        uint256 wadToSlash,
        string calldata description
    ) external;

    /**
     * @notice Deregister operator from operator sets
     * @param operator The operator to deregister
     * @param operatorSetIds Array of operator set IDs to deregister from
     */
    function deregisterOperatorFromOperatorSets(
        address operator,
        uint32[] memory operatorSetIds
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
     * @return priceFeed Price feed address for USD conversion
     */
    function getStrategyConfig(
        IStrategy strategy
    ) external view returns (address priceFeed);

    /**
     * @notice Get minimum collateral requirement
     * @return minCollateralUsd Minimum collateral in USD (18 decimals)
     */
    function getMinCollateralUsd()
        external
        view
        returns (uint256 minCollateralUsd);

    /**
     * @notice Get operator information (license and tickets)
     * @param operator Address of the operator
     * @return info OperatorInfo struct with license and ticket data
     */
    function getOperatorInfo(
        address operator
    ) external view returns (OperatorInfo memory info);

    /**
     * @notice Get current license stake requirement
     * @return licenseStake Amount of ENCL required for license
     */
    function getLicenseStake() external view returns (uint256 licenseStake);

    /**
     * @notice Get current ticket price
     * @return ticketPrice Price per ticket in USDC
     */
    function getTicketPrice() external view returns (uint256 ticketPrice);

    function getAvailableTicketBudget(
        address operator
    ) external view returns (uint256);
}
