// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IEnclave } from "./IEnclave.sol";
import { IBondingRegistry } from "./IBondingRegistry.sol";

/**
 * @title ICiphernodeRegistry
 * @notice Interface for managing ciphernode registration and committee selection
 * @dev This registry maintains an Incremental Merkle Tree (IMT) of registered ciphernodes
 * and coordinates committee selection for E3 computations
 */
interface ICiphernodeRegistry {
    /// @notice Tracks a committee member's lifecycle state for a given E3.
    enum MemberStatus {
        None,
        Active,
        Expelled
    }

    /// @notice Lifecycle stage of a committee for a given E3.
    enum CommitteeStage {
        None,
        Requested,
        Finalized,
        Failed
    }

    /// @notice Struct representing the sortition state for an E3 round.
    /// @param stage Current lifecycle stage of the committee (replaces former initialized/finalized/failed bools).
    /// @param requestBlock The block number when the committee was requested.
    /// @param committeeDeadline The deadline for committee formation (ticket submission).
    /// @param threshold The M/N threshold for the committee ([M, N]).
    /// @param publicKey Hash of the committee's public key.
    /// @param seed The seed for the round.
    /// @param topNodes Sorted top-N nodes selected during sortition.
    /// @param submitted Mapping of nodes to their submission status.
    /// @param scoreOf Mapping of nodes to their scores.
    /// @param memberStatus Tri-state membership tracking (None / Active / Expelled).
    struct Committee {
        CommitteeStage stage;
        uint256 seed;
        uint256 requestBlock;
        uint256 committeeDeadline;
        bytes32 publicKey;
        uint32[2] threshold;
        address[] topNodes;
        mapping(address node => bool submitted) submitted;
        mapping(address node => uint256 score) scoreOf;
        mapping(address node => MemberStatus) memberStatus;
        uint256 activeCount;
    }

    /// @notice This event MUST be emitted when a committee is selected for an E3.
    /// @param e3Id ID of the E3 for which the committee was selected.
    /// @param seed Random seed for score computation.
    /// @param threshold The M/N threshold for the committee.
    /// @param requestBlock Block number for snapshot validation.
    /// @param committeeDeadline Deadline for committee formation (ticket submission).
    event CommitteeRequested(
        uint256 indexed e3Id,
        uint256 seed,
        uint32[2] threshold,
        uint256 requestBlock,
        uint256 committeeDeadline
    );

    /// @notice This event MUST be emitted when a ticket is submitted for sortition
    /// @param e3Id ID of the E3 computation
    /// @param node Address of the ciphernode submitting the ticket
    /// @param ticketId The ticket number being submitted
    /// @param score The computed score for the ticket
    event TicketSubmitted(
        uint256 indexed e3Id,
        address indexed node,
        uint256 ticketId,
        uint256 score
    );

    /// @notice This event MUST be emitted when a committee is finalized
    /// @param e3Id ID of the E3 computation
    /// @param committee Array of selected ciphernode addresses
    event CommitteeFinalized(uint256 indexed e3Id, address[] committee);

    /// @notice This event MUST be emitted when committee formation fails (threshold not met)
    /// @param e3Id ID of the E3 computation
    /// @param nodesSubmitted Number of nodes that submitted tickets
    /// @param thresholdRequired Minimum number of nodes required
    event CommitteeFormationFailed(
        uint256 indexed e3Id,
        uint256 nodesSubmitted,
        uint256 thresholdRequired
    );

    /// @notice This event MUST be emitted when a committee is selected for an E3.
    /// @param e3Id ID of the E3 for which the committee was selected.
    /// @param publicKey Public key of the committee.
    event CommitteePublished(
        uint256 indexed e3Id,
        address[] nodes,
        bytes publicKey
    );

    /// @notice This event MUST be emitted when a committee's active status changes.
    /// @param e3Id ID of the E3 for which the committee status changed.
    /// @param active True if committee is now active, false if completed.
    event CommitteeActivationChanged(uint256 indexed e3Id, bool active);

    /// @notice This event MUST be emitted when a committee member is expelled due to slashing.
    /// @param e3Id ID of the E3 for which the member was expelled.
    /// @param node Address of the expelled committee member.
    /// @param reason Hash of the slash reason that caused the expulsion.
    /// @param activeCountAfter Number of active committee members remaining after expulsion.
    event CommitteeMemberExpelled(
        uint256 indexed e3Id,
        address indexed node,
        bytes32 reason,
        uint256 activeCountAfter
    );

    /// @notice This event MUST be emitted when committee viability changes after an expulsion.
    /// @param e3Id ID of the E3.
    /// @param activeCount Current number of active committee members.
    /// @param thresholdM The minimum threshold (M) required.
    /// @param viable Whether the committee is still viable (activeCount >= M).
    event CommitteeViabilityUpdated(
        uint256 indexed e3Id,
        uint256 activeCount,
        uint256 thresholdM,
        bool viable
    );

    /// @notice This event MUST be emitted when `enclave` is set.
    /// @param enclave Address of the enclave contract.
    event EnclaveSet(address indexed enclave);

    /// @notice This event MUST be emitted when a ciphernode is added to the registry.
    /// @param node Address of the ciphernode.
    /// @param index Index of the ciphernode in the registry.
    /// @param numNodes Number of ciphernodes in the registry.
    /// @param size Size of the registry.
    event CiphernodeAdded(
        address indexed node,
        uint256 index,
        uint256 numNodes,
        uint256 size
    );

    /// @notice This event MUST be emitted when a ciphernode is removed from the registry.
    /// @param node Address of the ciphernode.
    /// @param index Index of the ciphernode in the registry.
    /// @param numNodes Number of ciphernodes in the registry.
    /// @param size Size of the registry.
    event CiphernodeRemoved(
        address indexed node,
        uint256 index,
        uint256 numNodes,
        uint256 size
    );

    /// @notice This event MUST be emitted any time the `sortitionSubmissionWindow` is set.
    /// @param sortitionSubmissionWindow The submission window for the E3 sortition in seconds.
    event SortitionSubmissionWindowSet(uint256 sortitionSubmissionWindow);

    /// @notice Check if a ciphernode is eligible for committee selection
    /// @dev A ciphernode is eligible if it is enabled in the registry and meets bonding requirements
    /// @param ciphernode Address of the ciphernode to check
    /// @return eligible Whether the ciphernode is eligible for committee selection
    function isCiphernodeEligible(address ciphernode) external returns (bool);

    /// @notice Check if a ciphernode is enabled in the registry
    /// @param node Address of the ciphernode
    /// @return enabled Whether the ciphernode is enabled
    function isEnabled(address node) external view returns (bool enabled);

    /// @notice Add a ciphernode to the registry
    /// @param node Address of the ciphernode to add
    function addCiphernode(address node) external;

    /// @notice Remove a ciphernode from the registry
    /// @param node Address of the ciphernode to remove
    /// @param siblingNodes Array of sibling node indices for tree operations
    function removeCiphernode(
        address node,
        uint256[] calldata siblingNodes
    ) external;

    /// @notice Initiates the committee selection process for a specified E3.
    /// @dev This function MUST revert when not called by the Enclave contract.
    /// @param e3Id ID of the E3 for which to select the committee.
    /// @param seed Random seed for score computation.
    /// @param threshold The M/N threshold for the committee.
    /// @return success True if committee selection was successfully initiated.
    function requestCommittee(
        uint256 e3Id,
        uint256 seed,
        uint32[2] calldata threshold
    ) external returns (bool success);

    /// @notice Publishes the public key resulting from the committee selection process.
    /// @dev This function MUST revert if not called by the owner.
    /// @param e3Id ID of the E3 for which to select the committee.
    /// @param nodes Array of ciphernode addresses selected for the committee.
    /// @param publicKey The public key generated by the given committee.
    /// @param publicKeyHash The hash of the public key.
    function publishCommittee(
        uint256 e3Id,
        address[] calldata nodes,
        bytes calldata publicKey,
        bytes32 publicKeyHash
    ) external;

    /// @notice This function should be called by the Enclave contract to get the public key of a committee.
    /// @dev This function MUST revert if no committee has been requested for the given E3.
    /// @dev This function MUST revert if the committee has not yet published a public key.
    /// @param e3Id ID of the E3 for which to get the committee public key.
    /// @return publicKeyHash The hash of the public key of the given committee.
    function committeePublicKey(
        uint256 e3Id
    ) external view returns (bytes32 publicKeyHash);

    /// @notice This function should be called by the Enclave contract to get the committee for a given E3.
    /// @dev This function MUST revert if no committee has been requested for the given E3.
    /// @param e3Id ID of the E3 for which to get the committee.
    /// @return committeeNodes The nodes in the committee for the given E3.
    function getCommitteeNodes(
        uint256 e3Id
    ) external view returns (address[] memory committeeNodes);

    /// @notice Returns the current root of the ciphernode IMT
    /// @return Current IMT root
    function root() external view returns (uint256);

    /// @notice Returns the IMT root at the time a committee was requested
    /// @param e3Id ID of the E3
    /// @return IMT root at time of committee request
    function rootAt(uint256 e3Id) external view returns (uint256);

    /// @notice Returns the current size of the ciphernode IMT
    /// @return Size of the IMT
    function treeSize() external view returns (uint256);

    /// @notice Returns the address of the bonding registry
    /// @return Address of the bonding registry contract
    function getBondingRegistry() external view returns (address);

    /// @notice Sets the Enclave contract address
    /// @dev Only callable by owner
    /// @param _enclave Address of the Enclave contract
    function setEnclave(IEnclave _enclave) external;

    /// @notice Sets the bonding registry contract address
    /// @dev Only callable by owner
    /// @param _bondingRegistry Address of the bonding registry contract
    function setBondingRegistry(IBondingRegistry _bondingRegistry) external;

    /// @notice This function should be called to set the submission window for the E3 sortition.
    /// @param _sortitionSubmissionWindow The submission window for the E3 sortition in seconds.
    function setSortitionSubmissionWindow(
        uint256 _sortitionSubmissionWindow
    ) external;

    /// @notice Submit a ticket for sortition
    /// @dev Validates ticket against node's balance at request block
    /// @param e3Id ID of the E3 computation
    /// @param ticketNumber The ticket number to submit
    function submitTicket(uint256 e3Id, uint256 ticketNumber) external;

    /// @notice Finalize the committee after submission window closes
    /// @dev If threshold not met, marks E3 as failed and returns false
    /// @param e3Id ID of the E3 computation
    /// @return success True if committee formed successfully, false if threshold not met
    function finalizeCommittee(uint256 e3Id) external returns (bool success);

    /// @notice Check if submission window is still open for an E3
    /// @param e3Id ID of the E3 computation
    /// @return Whether the submission window is open
    function isOpen(uint256 e3Id) external view returns (bool);

    /// @notice Get the committee deadline for an E3
    /// @param e3Id ID of the E3 computation
    /// @return committeeDeadline The committee deadline timestamp
    function getCommitteeDeadline(uint256 e3Id) external view returns (uint256);

    /// @notice Expel a committee member from a specific E3 committee due to slashing
    /// @dev Only callable by SlashingManager. Idempotent (re-expelling same member is no-op).
    ///      Returns viability data so the caller can decide whether to fail the E3 —
    ///      eliminating the need for separate view calls to check count and threshold.
    /// @param e3Id ID of the E3 computation
    /// @param node Address of the committee member to expel
    /// @param reason Hash of the slash reason
    /// @return activeCount Number of active committee members after expulsion
    /// @return thresholdM The minimum threshold (M) required for viability
    function expelCommitteeMember(
        uint256 e3Id,
        address node,
        bytes32 reason
    ) external returns (uint256 activeCount, uint32 thresholdM);

    /// @notice Check if a committee member is still active for a specific E3
    /// @param e3Id ID of the E3 computation
    /// @param node Address of the committee member to check
    /// @return Whether the member is still active (not expelled) in the committee
    function isCommitteeMemberActive(
        uint256 e3Id,
        address node
    ) external view returns (bool);

    /// @notice Check if an address was ever a committee member for a specific E3
    /// @param e3Id ID of the E3 computation
    /// @param node Address to check
    /// @return Whether the address was ever a member of the finalized committee
    function isCommitteeMember(
        uint256 e3Id,
        address node
    ) external view returns (bool);

    /// @notice Get active (non-expelled) committee nodes for an E3
    /// @param e3Id ID of the E3 computation
    /// @return nodes Array of active committee member addresses
    function getActiveCommitteeNodes(
        uint256 e3Id
    ) external view returns (address[] memory nodes);

    /// @notice Consolidated committee viability check — avoids two separate view calls.
    /// @param e3Id ID of the E3 computation
    /// @return activeCount Current number of active (non-expelled) committee members
    /// @return thresholdM Minimum required members (M in M-of-N)
    /// @return thresholdN Total desired committee size (N in M-of-N)
    /// @return viable True when activeCount >= thresholdM
    function getCommitteeViability(
        uint256 e3Id
    )
        external
        view
        returns (
            uint256 activeCount,
            uint32 thresholdM,
            uint32 thresholdN,
            bool viable
        );
}
