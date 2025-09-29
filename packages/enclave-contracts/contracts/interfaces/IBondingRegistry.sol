// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity >=0.8.27;

import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";
import { ICiphernodeRegistry } from "./ICiphernodeRegistry.sol";
import { EnclaveTicketToken } from "../token/EnclaveTicketToken.sol";

/**
 * @title IBondingRegistry
 * @notice Interface for the main bonding registry that holds operator balance and license bonds
 */
interface IBondingRegistry {
    // ======================
    // Custom Errors
    // ======================

    // General
    error ZeroAddress();
    error ZeroAmount();
    error CiphernodeBanned();
    error Unauthorized();
    error InsufficientBalance();
    error NotLicensed();
    error AlreadyRegistered();
    error NotRegistered();
    error ExitInProgress();
    error ExitNotReady();
    error InvalidAmount();
    error InvalidConfiguration();
    error NoPendingDeregistration();
    error OnlyRewardDistributor();
    error ArrayLengthMismatch();

    // ======================
    // Events (Protocol-Named)
    // ======================

    /**
     * @notice Emitted when operator's ticket balance changes
     * @param operator Address of the operator
     * @param delta Change in balance (positive for increase, negative for decrease)
     * @param newBalance New total balance
     * @param reason Reason for the change (e.g., "DEPOSIT", "WITHDRAW", slash reason)
     */
    event TicketBalanceUpdated(
        address indexed operator,
        int256 delta,
        uint256 newBalance,
        bytes32 indexed reason
    );

    /**
     * @notice Emitted when operator's license bond changes
     * @param operator Address of the operator
     * @param delta Change in bond (positive for increase, negative for decrease)
     * @param newBond New total license bond
     * @param reason Reason for the change (e.g., "BOND", "UNBOND", slash reason)
     */
    event LicenseBondUpdated(
        address indexed operator,
        int256 delta,
        uint256 newBond,
        bytes32 indexed reason
    );

    /**
     * @notice Emitted when operator requests deregistration from the protocol
     * @param operator Address of the operator
     * @param unlockAt Timestamp when deregistration can be finalized
     */
    event CiphernodeDeregistrationRequested(
        address indexed operator,
        uint64 unlockAt
    );

    /**
     * @notice Emitted when operator active status changes
     * @param operator Address of the operator
     * @param active True if active, false if inactive
     */
    event OperatorActivationChanged(address indexed operator, bool active);

    /**
     * @notice Emitted when configuration is updated
     * @param parameter Name of the parameter
     * @param oldValue Previous value
     * @param newValue New value
     */
    event ConfigurationUpdated(
        bytes32 indexed parameter,
        uint256 oldValue,
        uint256 newValue
    );

    /**
     * @notice Emitted when treasury withdraws slashed funds
     * @param to Treasury address
     * @param ticketAmount Amount of slashed ticket balance withdrawn
     * @param licenseAmount Amount of slashed license bond withdrawn
     */
    event SlashedFundsWithdrawn(
        address indexed to,
        uint256 ticketAmount,
        uint256 licenseAmount
    );

    // ======================
    // View Functions
    // ======================

    /**
     * @notice Get operator's current ticket balance
     * @param operator Address of the operator
     * @return Current collateral balance
     */
    function getTicketBalance(address operator) external view returns (uint256);

    /**
     * @notice Get operator's current license bond
     * @param operator Address of the operator
     * @return Current license bond
     */
    function getLicenseBond(address operator) external view returns (uint256);

    /**
     * @notice Get current ticket price
     * @return Price per ticket in collateral token units
     */
    function ticketPrice() external view returns (uint256);

    /**
     * @notice Calculate available tickets for an operator
     * @param operator Address of the operator
     * @return Number of tickets available (floor(balance / ticketPrice))
     */
    function availableTickets(address operator) external view returns (uint256);

    /**
     * @notice Check if operator is licensed
     * @param operator Address of the operator
     * @return True if operator has sufficient license bond
     */
    function isLicensed(address operator) external view returns (bool);

    /**
     * @notice Check if operator is registered
     * @param operator Address of the operator
     * @return True if operator is registered
     */
    function isRegistered(address operator) external view returns (bool);

    /**
     * @notice Check if operator is active
     * @param operator Address of the operator
     * @return True if operator is active (licensed, registered, and has min tickets)
     */
    function isActive(address operator) external view returns (bool);

    /**
     * @notice Check if operator has deregistration in progress
     * @param operator Address of the operator
     * @return True if exit requested but not finalized
     */
    function hasExitInProgress(address operator) external view returns (bool);

    /**
     * @notice Get license bond price required
     * @return License bond price amount
     */
    function licenseRequiredBond() external view returns (uint256);

    /**
     * @notice Get minimum ticket balance required for activation
     * @return Minimum number of tickets required
     */
    function minTicketBalance() external view returns (uint256);

    /**
     * @notice Get exit delay period
     * @return Number of seconds operators must wait after requesting exit
     */
    function exitDelay() external view returns (uint64);

    /**
     * @notice Get slashed funds treasury address
     * @return Address where slashed funds are sent
     */
    function slashedFundsTreasury() external view returns (address);

    /**
     * @notice Get total slashed ticket balance
     * @return Amount of ticket balance slashed and available for treasury withdrawal
     */
    function slashedTicketBalance() external view returns (uint256);

    /**
     * @notice Get total slashed license bond
     * @return Amount of license bond slashed and available for treasury withdrawal
     */
    function slashedLicenseBond() external view returns (uint256);

    // ======================
    // Operator Functions
    // ======================

    /**
     * @notice Register as an operator (callable by licensed operators)
     * @dev Requires sufficient license bond and calls registry
     */
    function registerOperator() external;

    /**
     * @notice Deregister as an operator and remove from IMT
     * @param siblingNodes Sibling node proofs for IMT removal
     * @dev Requires operator to provide sibling nodes for immediate IMT removal
     */
    function deregisterOperator(uint256[] calldata siblingNodes) external;

    /**
     * @notice Increase operator's ticket balance by depositing tokens
     * @param amount Amount of ticket tokens to deposit
     * @dev Requires approval for ticket token transfer
     */
    function addTicketBalance(uint256 amount) external;

    /**
     * @notice Decrease operator's ticket balance by withdrawing tokens
     * @param amount Amount of ticket tokens to withdraw
     * @dev Reverts if operator is in any active committee
     */
    function removeTicketBalance(uint256 amount) external;

    /**
     * @notice Bond license tokens to become eligible for registration
     * @param amount Amount of license tokens to bond
     * @dev Requires approval for license token transfer
     */
    function bondLicense(uint256 amount) external;

    /**
     * @notice Unbond license tokens
     * @param amount Amount of license tokens to unbond
     * @dev Reverts if operator is in any active committee or still registered
     */
    function unbondLicense(uint256 amount) external;

    // ======================
    // Claim Functions
    // ======================

    /**
     * @notice Claim operator's ticket balance and license bond
     * @param maxTicketAmount Maximum amount of ticket tokens to claim
     * @param maxLicenseAmount Maximum amount of license tokens to claim
     */
    function claimExits(
        uint256 maxTicketAmount,
        uint256 maxLicenseAmount
    ) external;

    // ======================
    // Slashing Functions
    // ======================

    /**
     * @notice Slash operator's ticket balance by absolute amount
     * @param operator Address of the operator to slash
     * @param amount Amount to slash
     * @param reason Reason for slashing (stored in event)
     * @dev Only callable by authorized slashing manager
     */
    function slashTicketBalance(
        address operator,
        uint256 amount,
        bytes32 reason
    ) external;

    /**
     * @notice Slash operator's license bond by absolute amount
     * @param operator Address of the operator to slash
     * @param amount Amount to slash
     * @param reason Reason for slashing (stored in event)
     * @dev Only callable by authorized slashing manager
     */
    function slashLicenseBond(
        address operator,
        uint256 amount,
        bytes32 reason
    ) external;

    // ======================
    // Reward Distribution Functions
    // ======================
    /**
     * @notice Distribute rewards to operators
     * @param rewardToken Reward token contract
     * @param operators Addresses of the operators to distribute rewards to
     * @param amounts Amounts of rewards to distribute to each operator
     * @dev Only callable by contract owner
     */
    function distributeRewards(
        IERC20 rewardToken,
        address[] calldata operators,
        uint256[] calldata amounts
    ) external;

    // ======================
    // Admin Functions
    // ======================

    /**
     * @notice Set ticket price
     * @param newTicketPrice New price per ticket
     * @dev Only callable by contract owner
     */
    function setTicketPrice(uint256 newTicketPrice) external;

    /**
     * @notice Set license bond price required
     * @param newLicenseRequiredBond New license bond price
     * @dev Only callable by contract owner
     */
    function setLicenseRequiredBond(uint256 newLicenseRequiredBond) external;

    /**
     * @notice Set license active BPS
     * @param newBps New license active BPS
     * @dev Only callable by contract owner
     */
    function setLicenseActiveBps(uint256 newBps) external;

    /**
     * @notice Set minimum ticket balance required for activation
     * @param newMinTicketBalance New minimum ticket balance
     * @dev Only callable by contract owner
     */
    function setMinTicketBalance(uint256 newMinTicketBalance) external;

    /**
     * @notice Set exit delay period
     * @param newExitDelay New exit delay in seconds
     * @dev Only callable by contract owner
     */
    function setExitDelay(uint64 newExitDelay) external;

    /**
     * @notice Set ticket token
     * @param newTicketToken New ticket token
     * @dev Only callable by contract owner
     */
    function setTicketToken(EnclaveTicketToken newTicketToken) external;

    /**
     * @notice Set license token
     * @param newLicenseToken New license token
     * @dev Only callable by contract owner
     */
    function setLicenseToken(IERC20 newLicenseToken) external;

    /**
     * @notice Set slashed funds treasury address
     * @param newSlashedFundsTreasury New slashed funds treasury address
     * @dev Only callable by contract owner
     */
    function setSlashedFundsTreasury(address newSlashedFundsTreasury) external;

    /**
     * @notice Set registry address
     * @param newRegistry New registry contract address
     * @dev Only callable by contract owner
     */
    function setRegistry(ICiphernodeRegistry newRegistry) external;

    /**
     * @notice Set slashing manager address
     * @param newSlashingManager New slashing manager contract address
     * @dev Only callable by contract owner
     */
    function setSlashingManager(address newSlashingManager) external;

    /**
     * @notice Withdraw slashed funds to treasury
     * @param ticketAmount Amount of slashed ticket balance to withdraw
     * @param licenseAmount Amount of slashed license bond to withdraw
     * @dev Only callable by contract owner, sends to treasury address
     */
    function withdrawSlashedFunds(
        uint256 ticketAmount,
        uint256 licenseAmount
    ) external;
}
