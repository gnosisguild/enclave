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
    error InvalidMinTicketBalance();
    error InvalidTicketPrice();
    error TicketNotFound();
    error TicketAlreadyInactive();
    error InsufficientUSDCBalance();
    error InsufficientUSDCAllowance();

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
        uint32 ticketId,
        uint256 ticketCount,
        uint256 totalCost
    );
    event TicketUsed(address indexed operator, uint256 indexed ticketId);
    event TicketSlashed(
        address indexed operator,
        uint256 indexed ticketId,
        uint256 slashAmount
    );
    event TicketToppedUp(
        address indexed operator,
        uint32 ticketId,
        uint256 amount
    );
    event TicketStatusChanged(
        address indexed operator,
        uint32 ticketId,
        TicketStatus newStatus
    );
    event MinTicketBalanceUpdated(uint256 newBalance);
    event CiphernodeRegistered(address indexed operator);
    event CiphernodeDeregistered(address indexed operator);
    event CiphernodeActivated(address indexed operator);
    event CiphernodeDeactivated(address indexed operator);

    event LicenseStakeUpdated(uint256 newStake);
    event TicketPriceUpdated(uint256 newPrice);
    event USDCWithdrawn(address indexed to, uint256 amount);

    // Enums
    enum TicketStatus {
        Active,
        Inactive,
        Burned
    }

    // Structs
    struct Ticket {
        uint64 createdAt;
        uint96 originalValue;
        uint96 slashedAmount;
        address operator;
        uint32 id;
        bool isUsed;
        TicketStatus status;
    }

    struct OperatorInfo {
        bool isLicensed;
        uint256 licenseStake;
        uint256 registeredAt;
        bool isActive;
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
     * @notice Top up an existing ticket with more USDC
     * @param ticketId ID of the ticket to top up
     * @param usdcAmount Amount of USDC to add
     */
    function topUpTicket(uint32 ticketId, uint256 usdcAmount) external;

    /**
     * @notice Use a specific ticket for computation (callable by registry)
     * @param operator Address of the operator
     * @param ticketId ID of the ticket to use
     */
    function useTicket(address operator, uint32 ticketId) external;

    /**
     * @notice Slash a specific ticket (callable by ServiceManager)
     * @param operator Address of the operator
     * @param ticketId ID of the ticket to slash
     * @param wadToSlash Proportion to slash (in WAD format)
     */
    function slashTicket(
        address operator,
        uint32 ticketId,
        uint256 wadToSlash
    ) external;

    /**
     * @notice Slash all active tickets for an operator (callable by ServiceManager)
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
     * @notice Set minimum USDC balance across all tickets for activation
     * @param _minTicketBalance New minimum ticket balance in USDC
     */
    function setMinTicketBalance(uint256 _minTicketBalance) external;

    /**
     * @notice Set license stake requirement
     * @param _licenseStake New license stake amount
     */
    function setLicenseStake(uint256 _licenseStake) external;

    /**
     * @notice Set ticket price (cost per ticket)
     * @param _ticketPrice New ticket price in USDC
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
     * @notice Get total active ticket balance for operator
     * @param operator Address of the operator
     * @return Total USDC balance in active tickets
     */
    function getTotalActiveTicketBalance(
        address operator
    ) external view returns (uint256);

    /**
     * @notice Get specific ticket information
     * @param ticketId ID of the ticket
     * @return Ticket struct
     */
    function getTicket(uint32 ticketId) external view returns (Ticket memory);

    /**
     * @notice Get all ticket IDs for an operator
     * @param operator Address of the operator
     * @return Array of ticket IDs
     */
    function getOperatorTickets(
        address operator
    ) external view returns (uint32[] memory);

    /**
     * @notice Get available (unused and not too slashed) tickets for an operator
     * @param operator Address of the operator
     * @return Array of available ticket IDs
     */
    function getAvailableTickets(
        address operator
    ) external view returns (uint32[] memory);

    /**
     * @notice Get license stake requirement
     * @return License stake amount
     */
    function getLicenseStake() external view returns (uint256);

    /**
     * @notice Get current ticket price
     * @return Ticket price in USDC
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
