// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

/**
 * @title IE3Lifecycle
 * @notice Interface for E3 lifecycle state machine with timeout enforcement
 * @dev Tracks E3 progress through stages and enables failure detection
 */
interface IE3Lifecycle {
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                         Enums                          //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @notice Lifecycle stages of an E3 computation
    /// @dev Flow: Requested → CommitteeFinalized → KeyPublished → Activated → CiphertextReady → Complete
    ///      Any stage can transition to Failed on timeout
    enum E3Stage {
        None, // 0 - E3 doesn't exist
        Requested, // 1 - Payment locked, awaiting committee finalization (sortition)
        CommitteeFinalized, // 2 - Committee selected via sortition, DKG in progress
        KeyPublished, // 3 - DKG complete, public key published, awaiting activation
        Activated, // 4 - E3 active, accepting inputs until expiration
        CiphertextReady, // 5 - Computation done, encrypted output published, awaiting decryption
        Complete, // 6 - Terminal: Success
        Failed // 7 - Terminal: Failure
    }
    /// @notice Reasons why an E3 failed
    enum FailureReason {
        None,
        // Committee Formation
        CommitteeFormationTimeout, // No committee formed in time
        InsufficientCommitteeMembers, // Not enough nodes responded
        // DKG
        DKGTimeout, // DKG didn't complete in time
        DKGInvalidShares, // Malicious shares detected
        // Activation
        ActivationWindowExpired, // startWindow[1] passed without activation
        // Inputs
        NoInputsReceived, // Input window closed with no inputs
        // Computation
        ComputeTimeout, // Computation didn't complete in time
        ComputeProviderExpired, // Provider request expired (no lock)
        ComputeProviderFailed, // Provider locked but failed
        RequesterCancelled, // Requester chose to end during compute
        // Decryption
        DecryptionTimeout, // Not enough decryption shares in time
        DecryptionInvalidShares, // Invalid decryption shares
        VerificationFailed // Plaintext verification rejected
    }
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Structs                         //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @notice Timeout configuration for E3 stages
    struct E3TimeoutConfig {
        uint256 committeeFormationWindow; // Time for committee to form
        uint256 dkgWindow; // Time for DKG to complete
        uint256 computeWindow; // Time for FHE computation
        uint256 decryptionWindow; // Time for threshold decryption
        uint256 gracePeriod; // Buffer before slashing kicks in
    }
    /// @notice Deadlines for each E3
    struct E3Deadlines {
        uint256 committeeDeadline; // Deadline for committee formation
        uint256 dkgDeadline; // Deadline for DKG completion
        uint256 activationDeadline; // Deadline for activation (inputs must start by this time)
        uint256 computeDeadline; // Deadline for computation
        uint256 decryptionDeadline; // Deadline for decryption
    }
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Events                          //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @notice Emitted when E3 stage changes
    event E3StageChanged(
        uint256 indexed e3Id,
        E3Stage previousStage,
        E3Stage newStage
    );
    /// @notice Emitted when an E3 is marked as failed
    event E3Failed(
        uint256 indexed e3Id,
        E3Stage failedAtStage,
        FailureReason reason
    );
    /// @notice Emitted when timeout config is updated
    event TimeoutConfigUpdated(E3TimeoutConfig config);
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Errors                          //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @notice E3 is not in expected stage
    error InvalidStage(uint256 e3Id, E3Stage expected, E3Stage actual);
    /// @notice E3 has already been marked as failed
    error E3AlreadyFailed(uint256 e3Id);
    /// @notice E3 has already completed
    error E3AlreadyComplete(uint256 e3Id);
    /// @notice Failure condition not yet met
    error FailureConditionNotMet(uint256 e3Id);
    /// @notice Caller not authorized
    error Unauthorized();

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                      Functions                         //
    //                                                        //
    ////////////////////////////////////////////////////////////
    /// @notice Initialize E3 lifecycle (called by Enclave.request)
    /// @param e3Id The E3 ID
    /// @param requester The address that requested the E3
    function initializeE3(uint256 e3Id, address requester) external;

    /// @notice Transition to CommitteeFinalized stage (sortition complete, DKG starting)
    /// @dev Called when CiphernodeRegistry.finalizeCommittee() succeeds
    /// @param e3Id The E3 ID
    function onCommitteeFinalized(uint256 e3Id) external;

    /// @notice Transition to KeyPublished stage (DKG complete, public key ready)
    /// @dev Called when CiphernodeRegistry.publishCommittee() is called
    /// @param e3Id The E3 ID
    /// @param activationDeadline The deadline by which the E3 must be activated (startWindow[1])
    function onKeyPublished(uint256 e3Id, uint256 activationDeadline) external;

    /// @notice Transition to Activated stage
    /// @param e3Id The E3 ID
    /// @param expiration The expiration timestamp (when inputs close)
    function onActivated(uint256 e3Id, uint256 expiration) external;

    /// @notice Transition to CiphertextReady stage
    /// @param e3Id The E3 ID
    function onCiphertextPublished(uint256 e3Id) external;

    /// @notice Transition to Complete stage
    /// @param e3Id The E3 ID
    function onComplete(uint256 e3Id) external;

    /// @notice Anyone can mark an E3 as failed if timeout passed
    /// @param e3Id The E3 ID
    /// @return reason The failure reason
    function markE3Failed(uint256 e3Id) external returns (FailureReason reason);

    /// @notice Mark E3 as failed with specific reason (internal use)
    /// @param e3Id The E3 ID
    /// @param reason The failure reason
    function markE3FailedWithReason(
        uint256 e3Id,
        FailureReason reason
    ) external;

    /// @notice Get current stage of an E3
    /// @param e3Id The E3 ID
    /// @return stage The current stage
    function getE3Stage(uint256 e3Id) external view returns (E3Stage stage);

    /// @notice Get failure reason for an E3
    /// @param e3Id The E3 ID
    /// @return reason The failure reason
    function getFailureReason(
        uint256 e3Id
    ) external view returns (FailureReason reason);

    /// @notice Get requester address for an E3
    /// @param e3Id The E3 ID
    /// @return requester The requester address
    function getRequester(
        uint256 e3Id
    ) external view returns (address requester);

    /// @notice Get deadlines for an E3
    /// @param e3Id The E3 ID
    /// @return deadlines The E3 deadlines
    function getDeadlines(
        uint256 e3Id
    ) external view returns (E3Deadlines memory deadlines);

    /// @notice Check if E3 can be marked as failed
    /// @param e3Id The E3 ID
    /// @return canFail Whether failure condition is met
    /// @return reason The failure reason if applicable
    function checkFailureCondition(
        uint256 e3Id
    ) external view returns (bool canFail, FailureReason reason);

    /// @notice Set timeout configuration
    /// @param config The new timeout config
    function setTimeoutConfig(E3TimeoutConfig calldata config) external;

    /// @notice Get timeout configuration
    /// @return config The current timeout config
    function getTimeoutConfig()
        external
        view
        returns (E3TimeoutConfig memory config);
}
