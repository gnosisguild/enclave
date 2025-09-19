// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity >=0.8.27;

import { IBondingRegistry } from "./IBondingRegistry.sol";

/**
 * @title ISlashingManager
 * @notice Interface for managing slashing proposals, appeals, and execution
 * @dev Maintains policy table and handles slash workflows with appeals
 */
interface ISlashingManager {
    // ======================
    // Structs
    // ======================

    /**
     * @notice Slashing policy configuration for different slash reasons
     */
    struct SlashPolicy {
        uint256 ticketPenalty; // Amount or BPS for ticket collateral penalty
        uint256 licensePenalty; // Amount or BPS for license stake penalty
        bool useTicketBps; // True if ticket penalty is in BPS, false if absolute amount
        bool useLicenseBps; // True if license penalty is in BPS, false if absolute amount
        bool requiresProof; // True if slash requires verifier proof
        address proofVerifier; // Address of the verifier contract for proof verification
        bool banNode; // True if this slash type would result in banning
        uint256 appealWindow; // Seconds operators have to appeal (0 = immediate execution)
        bool enabled; // True if this slash type is currently enabled
    }

    /**
     * @notice Slash proposal details
     */
    struct SlashProposal {
        address operator; // Address being slashed
        bytes32 reason; // Reason hash (maps to SlashPolicy)
        uint256 ticketAmount; // Calculated ticket penalty amount
        uint256 licenseAmount; // Calculated license penalty amount
        bool executedTicket; // True if ticket penalty executed
        bool executedLicense; // True if license penalty executed
        bool appealed; // True if operator filed appeal
        bool resolved; // True if appeal was resolved
        bool approved; // True if appeal was approved (penalty cancelled)
        uint256 proposedAt; // Timestamp when proposed
        uint256 executableAt; // Timestamp when execution is allowed
        address proposer; // Address that proposed the slash
        bytes32 proofHash; // Hash of the proof data
        bool proofVerified; // True if proof was verified
    }

    // ======================
    // Errors
    // ======================

    error ZeroAddress();
    error Unauthorized();
    error InvalidPolicy();
    error InvalidProposal();
    error ProofRequired();
    error InvalidProof();
    error AppealWindowExpired();
    error AppealWindowActive();
    error AlreadyAppealed();
    error AlreadyExecuted();
    error AlreadyResolved();
    error SlashReasonNotFound();
    error SlashReasonDisabled();
    error CiphernodeBanned();

    // ======================
    // Events
    // ======================

    /**
     * @notice Emitted when a slash policy is updated
     */
    event SlashPolicyUpdated(bytes32 indexed reason, SlashPolicy policy);

    /**
     * @notice Emitted when a slash is proposed
     */
    event SlashProposed(
        uint256 indexed proposalId,
        address indexed operator,
        bytes32 indexed reason,
        uint256 ticketAmount,
        uint256 licenseAmount,
        uint256 executableAt,
        address proposer
    );

    /**
     * @notice Emitted when a slash is executed
     */
    event SlashExecuted(
        uint256 indexed proposalId,
        address indexed operator,
        bytes32 indexed reason,
        uint256 ticketAmount,
        uint256 licenseAmount,
        bool ticketExecuted,
        bool licenseExecuted
    );

    /**
     * @notice Emitted when an appeal is filed
     */
    event AppealFiled(
        uint256 indexed proposalId,
        address indexed operator,
        bytes32 indexed reason,
        string evidence
    );

    /**
     * @notice Emitted when an appeal is resolved
     */
    event AppealResolved(
        uint256 indexed proposalId,
        address indexed operator,
        bool approved,
        address resolver,
        string resolution
    );

    /**
     * @notice Emitted when a node is banned
     */
    event NodeBanned(
        address indexed node,
        bytes32 indexed reason,
        address banner
    );

    /**
     * @notice Emitted when a node is unbanned
     */
    event NodeUnbanned(address indexed node, address unbanner);

    // ======================
    // View Functions
    // ======================

    /**
     * @notice Get slash policy for a reason
     */
    function getSlashPolicy(
        bytes32 reason
    ) external view returns (SlashPolicy memory);

    /**
     * @notice Get slash proposal details
     */
    function getSlashProposal(
        uint256 proposalId
    ) external view returns (SlashProposal memory);

    /**
     * @notice Get total number of proposals
     */
    function totalProposals() external view returns (uint256);

    /**
     * @notice Check if a node is banned
     */
    function isBanned(address node) external view returns (bool);

    /**
     * @notice Get bonding vault contract
     */
    function bondingRegistry() external view returns (IBondingRegistry);

    // ======================
    // Admin Functions
    // ======================

    /**
     * @notice Set slash policy for a reason
     * @param reason Reason hash to set policy for
     * @param policy Policy configuration
     */
    function setSlashPolicy(
        bytes32 reason,
        SlashPolicy calldata policy
    ) external;

    /**
     * @notice Set bonding vault address
     * @param newBondingRegistry New bonding vault contract address
     */
    function setBondingRegistry(address newBondingRegistry) external;

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
     * @notice Add authorized verifier
     * @param verifier Address to authorize for proof verification
     */
    function addVerifier(address verifier) external;

    /**
     * @notice Remove authorized verifier
     * @param verifier Address to remove from verification authorization
     */
    function removeVerifier(address verifier) external;

    // ======================
    // Slashing Functions
    // ======================

    /**
     * @notice Propose a slash with proof
     * @param operator Address to slash
     * @param reason Slash reason (must have configured policy)
     * @param proof Proof data (if required by policy)
     * @return proposalId ID of the created proposal
     */
    function proposeSlash(
        address operator,
        bytes32 reason,
        bytes calldata proof
    ) external returns (uint256 proposalId);

    /**
     * @notice Execute a slash proposal
     * @param proposalId ID of the proposal to execute
     */
    function executeSlash(uint256 proposalId) external;

    // ======================
    // Appeal Functions
    // ======================

    /**
     * @notice File an appeal for a slash proposal
     * @param proposalId ID of the proposal to appeal
     * @param evidence Evidence string supporting the appeal
     */
    function fileAppeal(uint256 proposalId, string calldata evidence) external;

    /**
     * @notice Resolve an appeal (governance only)
     * @param proposalId ID of the proposal with appeal
     * @param approved True to approve appeal (cancel slash), false to deny
     * @param resolution Resolution explanation string
     */
    function resolveAppeal(
        uint256 proposalId,
        bool approved,
        string calldata resolution
    ) external;

    // ======================
    // Ban Management
    // ======================

    /**
     * @notice Ban a node (governance only)
     * @param node Address to ban
     * @param reason Reason for banning
     */
    function banNode(address node, bytes32 reason) external;

    /**
     * @notice Unban a node (governance only)
     * @param node Address to unban
     */
    function unbanNode(address node) external;

    /**
     * @notice Emergency pause slashing operations
     */
    function pause() external;

    /**
     * @notice Unpause slashing operations
     */
    function unpause() external;
}
