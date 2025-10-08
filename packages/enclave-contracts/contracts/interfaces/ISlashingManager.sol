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
     * @dev Defines penalties, proof requirements, and appeal mechanisms for each slash type
     * @param ticketPenalty Amount of ticket collateral to slash (in wei)
     * @param licensePenalty Amount of license bond to slash (in wei)
     * @param requiresProof Whether this slash type requires cryptographic proof verification
     * @param proofVerifier Address of the ISlashVerifier contract for proof validation
     * @param banNode Whether executing this slash will permanently ban the node
     * @param appealWindow Time window in seconds for operators to appeal (0 = immediate execution, no appeals)
     * @param enabled Whether this slash type is currently active and can be proposed
     */
    struct SlashPolicy {
        uint256 ticketPenalty;
        uint256 licensePenalty;
        bool requiresProof;
        address proofVerifier;
        bool banNode;
        uint256 appealWindow;
        bool enabled;
    }

    /**
     * @notice Slash proposal details tracking the full lifecycle of a slash
     * @dev Stores all state needed for proposal, appeal, and execution workflows
     * @param operator Address of the ciphernode operator being slashed
     * @param reason Hash of the slash reason (maps to SlashPolicy configuration)
     * @param ticketAmount Amount of ticket collateral to slash (copied from policy at proposal time)
     * @param licenseAmount Amount of license bond to slash (copied from policy at proposal time)
     * @param executed Whether the slashing penalties have been executed
     * @param appealed Whether the operator has filed an appeal
     * @param resolved Whether the appeal has been resolved by governance
     * @param appealUpheld Whether the appeal was approved (true = cancel slash, false = slash proceeds)
     * @param proposedAt Block timestamp when the slash was proposed
     * @param executableAt Block timestamp when execution becomes possible (proposedAt + appealWindow)
     * @param proposer Address that created this slash proposal
     * @param proofHash Keccak256 hash of the proof data submitted with the proposal
     * @param proofVerified Whether the proof was successfully verified by the proof verifier contract
     */
    struct SlashProposal {
        address operator;
        bytes32 reason;
        uint256 ticketAmount;
        uint256 licenseAmount;
        bool executed;
        bool appealed;
        bool resolved;
        bool appealUpheld;
        uint256 proposedAt;
        uint256 executableAt;
        address proposer;
        bytes32 proofHash;
        bool proofVerified;
    }

    // ======================
    // Errors
    // ======================

    /// @notice Thrown when a zero address is provided where a valid address is required
    error ZeroAddress();

    /// @notice Thrown when caller lacks required role permissions for the operation
    error Unauthorized();

    /// @notice Thrown when a slash policy configuration is invalid
    error InvalidPolicy();

    /// @notice Thrown when referencing a proposal ID that doesn't exist or is in invalid state
    error InvalidProposal();

    /// @notice Thrown when proof is required by policy but not provided
    error ProofRequired();

    /// @notice Thrown when provided proof fails verification
    error InvalidProof();

    /// @notice Thrown when attempting to execute a slash whose appeal was upheld
    error AppealUpheld();

    /// @notice Thrown when attempting to execute a slash with an unresolved appeal
    error AppealPending();

    /// @notice Thrown when attempting to file an appeal after the appeal window has closed
    error AppealWindowExpired();

    /// @notice Thrown when attempting to execute a slash before the appeal window has closed
    error AppealWindowActive();

    /// @notice Thrown when attempting to file a second appeal for the same proposal
    error AlreadyAppealed();

    /// @notice Thrown when attempting to execute a slash that has already been executed
    error AlreadyExecuted();

    /// @notice Thrown when attempting to resolve an appeal that has already been resolved
    error AlreadyResolved();

    /// @notice Thrown when referencing a slash reason that doesn't exist
    error SlashReasonNotFound();

    /// @notice Thrown when attempting to propose a slash for a disabled reason
    error SlashReasonDisabled();

    /// @notice Thrown when a banned ciphernode attempts a restricted operation
    error CiphernodeBanned();

    /// @notice Thrown when a policy requires proof but no verifier contract is configured
    error VerifierNotSet();

    // ======================
    // Events
    // ======================

    /**
     * @notice Emitted when a slash policy is created or updated
     * @param reason Hash of the slash reason being configured
     * @param policy The complete policy configuration including penalties and appeal settings
     */
    event SlashPolicyUpdated(bytes32 indexed reason, SlashPolicy policy);

    /**
     * @notice Emitted when a new slash proposal is created
     * @param proposalId Unique ID of the created proposal
     * @param operator Address of the ciphernode operator being slashed
     * @param reason Hash of the slash reason
     * @param ticketAmount Amount of ticket collateral to be slashed
     * @param licenseAmount Amount of license bond to be slashed
     * @param executableAt Timestamp when the slash can be executed (after appeal window)
     * @param proposer Address that created the proposal
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
     * @notice Emitted when a slash proposal is executed and penalties are applied
     * @param proposalId ID of the executed proposal
     * @param operator Address of the slashed operator
     * @param reason Hash of the slash reason
     * @param ticketAmount Amount of ticket collateral slashed
     * @param licenseAmount Amount of license bond slashed
     * @param executed Execution status (should always be true)
     */
    event SlashExecuted(
        uint256 indexed proposalId,
        address indexed operator,
        bytes32 indexed reason,
        uint256 ticketAmount,
        uint256 licenseAmount,
        bool executed
    );

    /**
     * @notice Emitted when an operator files an appeal against a slash proposal
     * @param proposalId ID of the proposal being appealed
     * @param operator Address of the operator filing the appeal
     * @param reason Hash of the slash reason being appealed
     * @param evidence Evidence string provided by the operator supporting their appeal
     */
    event AppealFiled(
        uint256 indexed proposalId,
        address indexed operator,
        bytes32 indexed reason,
        string evidence
    );

    /**
     * @notice Emitted when governance resolves an appeal
     * @param proposalId ID of the proposal with the resolved appeal
     * @param operator Address of the operator who appealed
     * @param appealUpheld Whether the appeal was approved (true = slash cancelled, false = slash proceeds)
     * @param resolver Address of the governance account that resolved the appeal
     * @param resolution Explanation string for the resolution decision
     */
    event AppealResolved(
        uint256 indexed proposalId,
        address indexed operator,
        bool appealUpheld,
        address resolver,
        string resolution
    );

    /**
     * @notice Emitted when a node is banned from the network
     * @param node Address of the banned node
     * @param reason Hash of the reason for banning
     * @param banner Address that executed the ban (governance or contract)
     */
    event NodeBanned(
        address indexed node,
        bytes32 indexed reason,
        address banner
    );

    /**
     * @notice Emitted when a previously banned node is unbanned
     * @param node Address of the unbanned node
     * @param unbanner Address of the governance account that unbanned the node
     */
    event NodeUnbanned(address indexed node, address unbanner);

    // ======================
    // View Functions
    // ======================

    /**
     * @notice Retrieves the slash policy configuration for a given reason
     * @param reason Hash of the slash reason to query
     * @return policy The complete SlashPolicy struct (returns default empty struct if not configured)
     */
    function getSlashPolicy(
        bytes32 reason
    ) external view returns (SlashPolicy memory policy);

    /**
     * @notice Retrieves the details of a slash proposal
     * @param proposalId ID of the proposal to query
     * @return proposal The complete SlashProposal struct
     * @dev Reverts with InvalidProposal if proposalId >= totalProposals
     */
    function getSlashProposal(
        uint256 proposalId
    ) external view returns (SlashProposal memory proposal);

    /**
     * @notice Returns the total number of slash proposals ever created
     * @return count The total count of proposals (next proposalId will be this value)
     */
    function totalProposals() external view returns (uint256 count);

    /**
     * @notice Checks whether a node is currently banned
     * @param node Address of the node to check
     * @return isBanned True if the node is banned, false otherwise
     */
    function isBanned(address node) external view returns (bool isBanned);

    /**
     * @notice Returns the bonding registry contract used for executing slashes
     * @return registry The IBondingRegistry contract instance
     */
    function bondingRegistry()
        external
        view
        returns (IBondingRegistry registry);

    // ======================
    // Admin Functions
    // ======================

    /**
     * @notice Creates or updates the slash policy for a specific reason
     * @dev Only callable by GOVERNANCE_ROLE. Validates policy constraints before setting
     * @param reason Hash of the slash reason to configure (must be non-zero)
     * @param policy Complete policy configuration including penalties, proof requirements, and appeal settings
     * Requirements:
     * - reason must not be bytes32(0)
     * - policy.enabled must be true
     * - At least one of ticketPenalty or licensePenalty must be non-zero
     * - If requiresProof is true, proofVerifier must be set and appealWindow must be 0
     * - If requiresProof is false, appealWindow must be greater than 0
     */
    function setSlashPolicy(
        bytes32 reason,
        SlashPolicy calldata policy
    ) external;

    /**
     * @notice Updates the bonding registry contract address
     * @dev Only callable by DEFAULT_ADMIN_ROLE. Used to execute actual slashing of funds
     * @param newBondingRegistry Address of the new IBondingRegistry contract (must be non-zero)
     */
    function setBondingRegistry(address newBondingRegistry) external;

    /**
     * @notice Grants SLASHER_ROLE to an address
     * @dev Only callable by DEFAULT_ADMIN_ROLE. Slashers can propose and execute slashes
     * @param slasher Address to grant slashing permissions (must be non-zero)
     */
    function addSlasher(address slasher) external;

    /**
     * @notice Revokes SLASHER_ROLE from an address
     * @dev Only callable by DEFAULT_ADMIN_ROLE
     * @param slasher Address to revoke slashing permissions from
     */
    function removeSlasher(address slasher) external;

    /**
     * @notice Grants VERIFIER_ROLE to an address
     * @dev Only callable by DEFAULT_ADMIN_ROLE. Verifiers can validate proof-based slashes
     * @param verifier Address to grant verification permissions (must be non-zero)
     */
    function addVerifier(address verifier) external;

    /**
     * @notice Revokes VERIFIER_ROLE from an address
     * @dev Only callable by DEFAULT_ADMIN_ROLE
     * @param verifier Address to revoke verification permissions from
     */
    function removeVerifier(address verifier) external;

    // ======================
    // Slashing Functions
    // ======================

    /**
     * @notice Creates a new slash proposal against an operator
     * @dev Only callable by SLASHER_ROLE. Validates policy and proof if required
     * @param operator Address of the ciphernode operator to slash (must be non-zero)
     * @param reason Hash of the slash reason (must have an enabled policy configured)
     * @param proof Proof data to be verified (required if policy.requiresProof is true, can be empty otherwise)
     * @return proposalId Sequential ID of the created proposal
     * Requirements:
     * - operator must not be zero address
     * - reason must have an enabled policy configured
     * - If policy requires proof, proof must be non-empty and pass verification
     * - Caller must have SLASHER_ROLE
     */
    function proposeSlash(
        address operator,
        bytes32 reason,
        bytes calldata proof
    ) external returns (uint256 proposalId);

    /**
     * @notice Executes a slash proposal and applies penalties to the operator
     * @dev Only callable by SLASHER_ROLE. Validates execution conditions and applies slashing
     * @param proposalId ID of the proposal to execute (must exist and not be already executed)
     * Requirements:
     * - Proposal must exist and not be already executed
     * - For proof-required slashes: proof must be verified
     * - For evidence-based slashes: appeal window must have expired
     * - If appeal was filed and resolved, appeal must not have been upheld
     * - Caller must have SLASHER_ROLE
     * Effects:
     * - Marks proposal as executed
     * - Slashes ticket balance if ticketAmount > 0
     * - Slashes license bond if licenseAmount > 0
     * - Bans node if policy.banNode is true
     */
    function executeSlash(uint256 proposalId) external;

    // ======================
    // Appeal Functions
    // ======================

    /**
     * @notice Allows an operator to file an appeal against a slash proposal
     * @dev Only the operator being slashed can file an appeal, and only within the appeal window
     * @param proposalId ID of the proposal to appeal (must exist)
     * @param evidence String containing evidence and arguments supporting the appeal
     * Requirements:
     * - Proposal must exist
     * - Caller must be the operator being slashed
     * - Current timestamp must be before proposal.executableAt (within appeal window)
     * - Proposal must not already have an appeal filed
     */
    function fileAppeal(uint256 proposalId, string calldata evidence) external;

    /**
     * @notice Resolves an appeal by accepting or rejecting it
     * @dev Only callable by GOVERNANCE_ROLE. If appeal is upheld, the slash cannot be executed
     * @param proposalId ID of the proposal with the appeal to resolve (must exist and have an appeal)
     * @param appealUpheld True to uphold the appeal (cancel the slash), false to deny the appeal
     *                     (allow slash to proceed)
     * @param resolution String explaining the governance decision
     * Requirements:
     * - Proposal must exist and have an appeal filed
     * - Appeal must not already be resolved
     * - Caller must have GOVERNANCE_ROLE
     * Effects:
     * - Marks appeal as resolved
     * - Sets appealUpheld flag (true = slash cancelled, false = slash can proceed)
     */
    function resolveAppeal(
        uint256 proposalId,
        bool appealUpheld,
        string calldata resolution
    ) external;

    // ======================
    // Ban Management
    // ======================

    /**
     * @notice Bans a node from the network
     * @dev Only callable by GOVERNANCE_ROLE. Bans can also occur automatically via executeSlash
     * @param node Address of the node to ban (must be non-zero)
     * @param reason Hash of the reason for banning
     * Requirements:
     * - node must not be zero address
     * - Caller must have GOVERNANCE_ROLE
     * Effects:
     * - Sets banned[node] to true
     * - Emits NodeBanned event
     */
    function banNode(address node, bytes32 reason) external;

    /**
     * @notice Removes a ban from a previously banned node
     * @dev Only callable by GOVERNANCE_ROLE
     * @param node Address of the node to unban (must be non-zero)
     * Requirements:
     * - node must not be zero address
     * - Caller must have GOVERNANCE_ROLE
     * Effects:
     * - Sets banned[node] to false
     * - Emits NodeUnbanned event
     */
    function unbanNode(address node) external;
}
