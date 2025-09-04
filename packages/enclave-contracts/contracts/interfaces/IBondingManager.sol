// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity >=0.8.27;

/**
 * @title IBondingManager
 * @notice Interface for managing operator licenses, tickets, and ciphernode registration
 */
interface IBondingManager {
    // ======================
    // General validation
    // ======================
    error ZeroAddress();

    // ======================
    // Operator-related
    // ======================
    error OperatorNotRegistered();
    error AlreadyRegistered();

    // ======================
    // Licensing-related
    // ======================
    error AlreadyLicensed();
    error NotLicensed();
    error InsufficientLicenseStake();

    // ======================
    // Ticket-related
    // ======================
    error InsufficientTicketBalance();
    error InvalidTicketAmount();
    error InsufficientTicketBudget();
    error InvalidMinTicketBalance();

    // ======================
    // Magnitude / resources
    // ======================
    error InsufficientAllocatedMagnitude();

    // ======================
    // Role/authorization
    // ======================
    error OnlyRegistry();
    error OnlyServiceManager();

    // Events
    event LicenseAcquired(address indexed operator, uint256 stake);
    event LicenseRevoked(address indexed operator);
    event TicketsPurchased(
        address indexed operator,
        uint256 cost,
        uint256 count
    );
    event TicketsUsed(address indexed operator, uint256 count);
    event TicketsSlashed(address indexed operator, uint256 count);
    event MinTicketBalanceUpdated(uint256 newBalance);
    event CiphernodeRegistered(address indexed operator, uint256 collateralUsd);
    event CiphernodeDeregistered(address indexed operator);
    event CiphernodeActivated(address indexed operator);
    event CiphernodeDeactivated(address indexed operator);

    event LicenseStakeUpdated(uint256 newStake);
    event TicketPriceUpdated(uint256 newPrice);

    // Structs
    struct OperatorInfo {
        bool isLicensed;
        uint256 licenseStake;
        uint256 ticketBalance;
        uint256 registeredAt;
        bool isActive;
        uint256 collateralUsd;
    }

    /**
     * @notice Acquire license to operate as ciphernode - this requires allocated magnitude and collateral
     */
    function acquireLicense() external;

    /**
     * @notice Purchase tickets for computation budget
     * @param ticketCount Number of tickets to purchase
     */
    function purchaseTickets(uint256 ticketCount) external;

    /**
     * @notice Register as ciphernode in the registry
     */
    function registerCiphernode() external;

    /**
     * @notice Deregister from ciphernode registry
     * @param siblingNodes Array of sibling node IDs for registry update
     */
    function deregisterCiphernode(uint256[] calldata siblingNodes) external;

    /**
     * @notice Use tickets for computation (callable by registry)
     * @param operator Address of the operator
     * @param ticketCount Number of tickets to use
     */
    function useTickets(address operator, uint256 ticketCount) external;

    /**
     * @notice Slash operator tickets proportionally (callable by ServiceManager)
     * @param operator Address of the operator
     * @param wadToSlash Proportion to slash (in WAD format)
     */
    function slashTickets(address operator, uint256 wadToSlash) external;

    /**
     * @notice Update license status after slashing (callable by ServiceManager)
     * @param operator Address of the operator
     */
    function updateLicenseStatus(address operator) external;

    /**
     * @notice Sync ticket health based on allocation changes (called by ServiceManager)
     * @param operator Address of the operator
     */
    function syncTicketHealth(address operator) external;

    /**
     * @notice Set minimum ticket balance for activation
     * @param _minTicketBalance New minimum ticket balance
     */
    function setMinTicketBalance(uint256 _minTicketBalance) external;

    /**
     * @notice Set license stake requirement
     * @param _licenseStake New license stake amount
     */
    function setLicenseStake(uint256 _licenseStake) external;

    /**
     * @notice Set ticket price
     * @param _ticketPrice New ticket price
     */
    function setTicketPrice(uint256 _ticketPrice) external;

    /**
     * @notice Get operator information
     * @param operator Address of the operator
     * @return OperatorInfo struct
     */
    function getOperatorInfo(
        address operator
    ) external view returns (OperatorInfo memory);

    /**
     * @notice Get available ticket budget for operator
     * @param operator Address of the operator
     * @return Available budget in USDC
     */
    function getAvailableTicketBudget(
        address operator
    ) external view returns (uint256);

    /**
     * @notice Get license stake requirement
     * @return License stake amount
     */
    function getLicenseStake() external view returns (uint256);

    /**
     * @notice Get ticket price
     * @return Ticket price
     */
    function getTicketPrice() external view returns (uint256);

    /**
     * @notice Check if operator is registered in ciphernode registry
     * @param operator Address of the operator
     * @return True if registered
     */
    function isRegisteredOperator(
        address operator
    ) external view returns (bool);

    /**
     * @notice Check if operator is active (registered and has enough tickets)
     * @param operator Address of the operator
     * @return True if active
     */
    function isActive(address operator) external view returns (bool);

    /**
     * @notice Get total USDC value spent on tickets by operator
     * @param operator Address of the operator
     * @return Total USDC spent on tickets
     */
    function ticketBudgetSpent(
        address operator
    ) external view returns (uint256);

    /**
     * @notice Ciphernode state enum for lifecycle management
     */
    enum CiphernodeState {
        REMOVED,
        REGISTERED_INACTIVE,
        ACTIVE
    }

    /**
     * @notice Get the current state of a ciphernode
     * @param operator Address of the operator
     * @return Current state of the ciphernode
     */
    function getCiphernodeState(
        address operator
    ) external view returns (CiphernodeState);
}
