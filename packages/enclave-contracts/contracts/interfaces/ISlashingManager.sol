// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

pragma solidity >=0.8.27;

import { IBondingRegistry } from "./IBondingRegistry.sol";
import { IE3RefundManager } from "./IE3RefundManager.sol";

/**
 * @title ISlashingManager
 * @notice Interface for managing slashing proposals, appeals, and execution
 * @dev Maintains policy table and handles slash workflows with two lanes:
 *      Lane A (proof-based): permissionless, atomic, no appeals
 *      Lane B (evidence-based): SLASHER_ROLE required, appeal window
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
     * @param affectsCommittee Whether executing this slash triggers committee expulsion for the target E3
     * @param failureReason The FailureReason enum value to use when committee drops below threshold (0 = no E3 failure)
     */
    struct SlashPolicy {
        uint256 ticketPenalty;
        uint256 licensePenalty;
        bool requiresProof;
        address proofVerifier;
        bool banNode;
        uint256 appealWindow;
        bool enabled;
        bool affectsCommittee;
        uint8 failureReason;
    }

    /**
     * @notice Slash proposal details tracking the full lifecycle of a slash
     * @dev Stores all state needed for proposal, appeal, and execution workflows
     * @param e3Id ID of the E3 computation this slash relates to (0 for non-E3 slashes)
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
        uint256 e3Id;
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
        /// @dev Snapshotted from SlashPolicy at proposal time to prevent execution drift
        bool banNode;
        /// @dev Snapshotted from SlashPolicy at proposal time to prevent execution drift
        bool affectsCommittee;
        /// @dev Snapshotted from SlashPolicy at proposal time to prevent execution drift
        uint8 failureReason;
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

    /// @notice The ZK proof verified successfully — the operator's submission was valid, not a fault
    error ProofIsValid();

    /// @notice Thrown when the recovered signer does not match the operator being slashed
    error SignerIsNotOperator();

    /// @notice Thrown when the operator is not a member of the committee for this E3
    error OperatorNotInCommittee();

    /// @notice Thrown when the verifier address in signed evidence doesn't match the policy's current verifier
    error VerifierMismatch();

    /// @notice Thrown when the verifier staticcall fails (e.g., contract doesn't exist, reverts, or runs out of gas)
    error VerifierCallFailed();

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

    /// @notice Thrown when the same evidence bundle has already been used in a proposal
    error DuplicateEvidence();

    /// @notice Thrown when the chainId in the signed proof payload does not match the current chain
    error ChainIdMismatch();

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
     * @param e3Id ID of the E3 computation related to this slash
     * @param operator Address of the ciphernode operator being slashed
     * @param reason Hash of the slash reason
     * @param ticketAmount Amount of ticket collateral to be slashed
     * @param licenseAmount Amount of license bond to be slashed
     * @param executableAt Timestamp when the slash can be executed (after appeal window)
     * @param proposer Address that created the proposal
     */
    event SlashProposed(
        uint256 indexed proposalId,
        uint256 indexed e3Id,
        address indexed operator,
        bytes32 reason,
        uint256 ticketAmount,
        uint256 licenseAmount,
        uint256 executableAt,
        address proposer
    );

    /**
     * @notice Emitted when a slash proposal is executed and penalties are applied
     * @param proposalId ID of the executed proposal
     * @param e3Id ID of the E3 committee associated with this slash
     * @param operator Address of the slashed operator
     * @param reason Hash of the slash reason
     * @param ticketAmount Amount of ticket collateral slashed
     * @param licenseAmount Amount of license bond slashed
     * @param executed Execution status (should always be true)
     */
    event SlashExecuted(
        uint256 indexed proposalId,
        uint256 e3Id,
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
     * @notice Emitted when a node is banned or unbanned from the network
     * @param node Address of the node
     * @param status Whether the node is banned
     * @param reason Hash of the reason for banning or unbanning
     * @param updater Address that executed the ban (governance or contract)
     */
    event NodeBanUpdated(
        address indexed node,
        bool status,
        bytes32 indexed reason,
        address updater
    );

    /**
     * @notice Emitted when slashed ticket funds are escrowed in the E3 refund pool
     * @param e3Id ID of the E3 computation
     * @param amount Amount of slashed funds escrowed (underlying stablecoin)
     */
    event SlashedFundsEscrowedToRefund(uint256 indexed e3Id, uint256 amount);

    /**
     * @notice Emitted when routing slashed funds fails (funds remain in BondingRegistry)
     * @param e3Id ID of the E3 computation
     * @param amount Amount that failed to route
     */
    event RoutingFailed(uint256 indexed e3Id, uint256 amount);

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
     * @notice Updates the bonding registry contract
     * @dev Only callable by DEFAULT_ADMIN_ROLE. Used to execute actual slashing of funds
     * @param newBondingRegistry The new IBondingRegistry contract (must be non-zero)
     */
    function setBondingRegistry(IBondingRegistry newBondingRegistry) external;

    /**
     * @notice Updates the E3 Refund Manager contract
     * @dev Only callable by DEFAULT_ADMIN_ROLE
     * @param newRefundManager The new IE3RefundManager contract (must be non-zero)
     */
    function setE3RefundManager(IE3RefundManager newRefundManager) external;

    /**
     * @notice Grants SLASHER_ROLE to an address
     * @dev Only callable by DEFAULT_ADMIN_ROLE. Slashers can propose and execute evidence-based slashes
     * @param slasher Address to grant slashing permissions (must be non-zero)
     */
    function addSlasher(address slasher) external;

    /**
     * @notice Revokes SLASHER_ROLE from an address
     * @dev Only callable by DEFAULT_ADMIN_ROLE
     * @param slasher Address to revoke slashing permissions from
     */
    function removeSlasher(address slasher) external;

    // ======================
    // Slashing Functions
    // ======================

    /**
     * @notice Creates a new slash proposal with cryptographic proof (Lane A - permissionless)
     * @dev Anyone can call this for proof-based slashes. Requires the operator's ECDSA signature
     *      over the proof payload to prevent arbitrary slashing.
     *      Evidence format:
     *        abi.encode(bytes zkProof, bytes32[] publicInputs,
     *        bytes signature, uint256 chainId, uint256 proofType, address verifier)
     *      The operator must have signed: keccak256(abi.encode(PROOF_PAYLOAD_TYPEHASH, chainId, e3Id,
     *        proofType, keccak256(zkProof), keccak256(abi.encodePacked(publicInputs))))
     *      Verifications performed:
     *        1. Verifier address in evidence matches the policy's current proofVerifier
     *        2. Signature recovery confirms the operator authored the bad proof
     *        3. Committee membership check confirms the operator was in the E3's committee
     *        4. ZK proof re-verification confirms the proof is indeed invalid (fault)
     * @param e3Id ID of the E3 computation this slash relates to
     * @param operator Address of the ciphernode operator to slash (must be non-zero)
     * @param reason Hash of the slash reason (must have an enabled proof-required policy)
     * @param proof Evidence data: abi.encode(zkProof, publicInputs, signature, chainId, proofType, verifier)
     * @return proposalId Sequential ID of the created proposal
     */
    function proposeSlash(
        uint256 e3Id,
        address operator,
        bytes32 reason,
        bytes calldata proof
    ) external returns (uint256 proposalId);

    /**
     * @notice Creates a new slash proposal with evidence (Lane B - SLASHER_ROLE required)
     * @dev Only callable by SLASHER_ROLE. Evidence-based slashes have appeal windows.
     * @param e3Id ID of the E3 computation this slash relates to
     * @param operator Address of the ciphernode operator to slash (must be non-zero)
     * @param reason Hash of the slash reason (must have an enabled non-proof policy)
     * @param evidence Evidence data supporting the slash proposal
     * @return proposalId Sequential ID of the created proposal
     */
    function proposeSlashEvidence(
        uint256 e3Id,
        address operator,
        bytes32 reason,
        bytes calldata evidence
    ) external returns (uint256 proposalId);

    /**
     * @notice Executes a slash proposal and applies penalties to the operator
     * @dev For evidence-based slashes, validates appeal window has expired.
     *      Proof-based slashes are executed atomically in proposeSlash.
     * @param proposalId ID of the proposal to execute (must exist and not be already executed)
     */
    function executeSlash(uint256 proposalId) external;

    /**
     * @notice Atomically redirects slashed ticket funds to E3RefundManager escrow
     * @dev Only callable by this contract (self-call pattern for try/catch atomicity).
     *      Transfers underlying stablecoin from BondingRegistry to E3RefundManager
     *      and calls Enclave.escrowSlashedFunds to update the escrow balance.
     * @param e3Id ID of the E3 computation
     * @param amount Amount of slashed ticket balance to escrow
     */
    function escrowSlashedFundsToRefund(uint256 e3Id, uint256 amount) external;

    // ======================
    // Appeal Functions
    // ======================

    /**
     * @notice Allows an operator to file an appeal against an evidence-based slash proposal
     * @dev Only the operator being slashed can file an appeal, and only within the appeal window
     * @param proposalId ID of the proposal to appeal (must exist)
     * @param evidence String containing evidence and arguments supporting the appeal
     */
    function fileAppeal(uint256 proposalId, string calldata evidence) external;

    /**
     * @notice Resolves an appeal by accepting or rejecting it
     * @dev Only callable by GOVERNANCE_ROLE. If appeal is upheld, the slash cannot be executed
     * @param proposalId ID of the proposal with the appeal to resolve
     * @param appealUpheld True to uphold the appeal (cancel the slash), false to deny
     * @param resolution String explaining the governance decision
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
     * @notice Bans or unbans a node from the network
     * @dev Only callable by GOVERNANCE_ROLE. Bans can also occur automatically via executeSlash
     * @param node Address of the node to ban (must be non-zero)
     * @param status Whether to ban the node
     * @param reason Hash of the reason for banning
     */
    function updateBanStatus(
        address node,
        bool status,
        bytes32 reason
    ) external;
}
