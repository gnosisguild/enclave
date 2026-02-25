// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { E3, IE3Program } from "./IE3.sol";
import { ICiphernodeRegistry } from "./ICiphernodeRegistry.sol";
import { IBondingRegistry } from "./IBondingRegistry.sol";
import { IDecryptionVerifier } from "./IDecryptionVerifier.sol";
import { IERC20 } from "@openzeppelin/contracts/token/ERC20/IERC20.sol";

interface IEnclave {
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                         Enums                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Lifecycle stages of an E3 computation
    enum E3Stage {
        None,
        Requested,
        CommitteeFinalized,
        KeyPublished,
        CiphertextReady,
        Complete,
        Failed
    }

    /// @notice Reasons why an E3 failed
    /// @dev Any new failure reason should be added before _MAX_FAILURE_REASON.
    enum FailureReason {
        None,
        CommitteeFormationTimeout,
        InsufficientCommitteeMembers,
        DKGTimeout,
        DKGInvalidShares,
        NoInputsReceived,
        ComputeTimeout,
        ComputeProviderExpired,
        ComputeProviderFailed,
        RequesterCancelled,
        DecryptionTimeout,
        DecryptionInvalidShares,
        VerificationFailed,
        _MAX_FAILURE_REASON
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Structs                         //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Timeout configuration for E3 stages
    struct E3TimeoutConfig {
        uint256 dkgWindow;
        uint256 computeWindow;
        uint256 decryptionWindow;
    }

    /// @notice Deadlines for each E3
    struct E3Deadlines {
        uint256 dkgDeadline;
        uint256 computeDeadline;
        uint256 decryptionDeadline;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                         Events                         //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice This event MUST be emitted when an Encrypted Execution Environment (E3) is successfully requested.
    /// @param e3Id ID of the E3.
    /// @param e3 Details of the E3.
    /// @param e3Program Address of the Computation module selected.
    event E3Requested(uint256 e3Id, E3 e3, IE3Program indexed e3Program);

    /// @notice This event MUST be emitted when an input to an Encrypted Execution Environment (E3) is
    /// successfully published.
    /// @param e3Id ID of the E3.
    /// @param data ABI encoded input data.
    event InputPublished(
        uint256 indexed e3Id,
        bytes data,
        uint256 inputHash,
        uint256 index
    );

    /// @notice This event MUST be emitted when the plaintext output of an Encrypted Execution Environment (E3)
    /// is successfully published.
    /// @param e3Id ID of the E3.
    /// @param plaintextOutput ABI encoded plaintext output.
    event PlaintextOutputPublished(uint256 indexed e3Id, bytes plaintextOutput);

    /// @notice This event MUST be emitted when the ciphertext output of an Encrypted Execution Environment (E3)
    /// is successfully published.
    /// @param e3Id ID of the E3.
    /// @param ciphertextOutput ABI encoded ciphertext output.
    event CiphertextOutputPublished(
        uint256 indexed e3Id,
        bytes ciphertextOutput
    );

    /// @notice This event MUST be emitted any time the `maxDuration` is set.
    /// @param maxDuration The maximum duration of a computation in seconds.
    event MaxDurationSet(uint256 maxDuration);

    /// @notice This event MUST be emitted any time the CiphernodeRegistry is set.
    /// @param ciphernodeRegistry The address of the CiphernodeRegistry contract.
    event CiphernodeRegistrySet(address ciphernodeRegistry);

    /// @notice This event MUST be emitted any time the BondingRegistry is set.
    /// @param bondingRegistry The address of the BondingRegistry contract.
    event BondingRegistrySet(address bondingRegistry);

    /// @notice This event MUST be emitted any time the fee token is set.
    /// @param feeToken The address of the fee token.
    event FeeTokenSet(address feeToken);

    /// @notice This event MUST be emitted when rewards are distributed to committee members.
    /// @param e3Id The ID of the E3 computation.
    /// @param nodes The addresses of the committee members receiving rewards.
    /// @param amounts The reward amounts for each committee member.
    event RewardsDistributed(
        uint256 indexed e3Id,
        address[] nodes,
        uint256[] amounts
    );

    /// @notice The event MUST be emitted any time an encryption scheme is enabled.
    /// @param encryptionSchemeId The ID of the encryption scheme that was enabled.
    event EncryptionSchemeEnabled(bytes32 encryptionSchemeId);

    /// @notice This event MUST be emitted any time an encryption scheme is disabled.
    /// @param encryptionSchemeId The ID of the encryption scheme that was disabled.
    event EncryptionSchemeDisabled(bytes32 encryptionSchemeId);

    /// @notice This event MUST be emitted any time a E3 Program is enabled.
    /// @param e3Program The address of the E3 Program.
    event E3ProgramEnabled(IE3Program e3Program);

    /// @notice This event MUST be emitted any time a E3 Program is disabled.
    /// @param e3Program The address of the E3 Program.
    event E3ProgramDisabled(IE3Program e3Program);

    /// @notice Emitted when the allowed E3 encryption scheme parameters are configured.
    /// @param e3ProgramParams Array of encoded encryption scheme parameters (e.g, for BFV)
    event AllowedE3ProgramsParamsSet(bytes[] e3ProgramParams);

    /// @notice Emitted when E3 program parameter sets are removed.
    /// @param e3ProgramParams Array of removed encryption scheme parameters.
    event E3ProgramsParamsRemoved(bytes[] e3ProgramParams);

    /// @notice Emitted when E3RefundManager contract is set.
    /// @param e3RefundManager The address of the E3RefundManager contract.
    event E3RefundManagerSet(address indexed e3RefundManager);

    /// @notice Emitted when the SlashingManager contract is set.
    /// @param slashingManager The address of the SlashingManager contract.
    event SlashingManagerSet(address indexed slashingManager);

    /// @notice Emitted when slashed funds are escrowed for an E3
    /// @param e3Id The E3 ID.
    /// @param amount The amount of slashed funds escrowed.
    event SlashedFundsEscrowed(uint256 indexed e3Id, uint256 amount);

    /// @notice Emitted when a failed E3 is processed for refunds.
    /// @param e3Id The ID of the failed E3.
    /// @param paymentAmount The original payment amount being refunded.
    /// @param honestNodeCount The number of honest nodes in the refund distribution.
    event E3FailureProcessed(
        uint256 indexed e3Id,
        uint256 paymentAmount,
        uint256 honestNodeCount
    );

    /// @notice Emitted when a committee is published and E3 lifecycle is updated.
    /// @param e3Id The ID of the E3.
    event CommitteeFormed(uint256 indexed e3Id);

    /// @notice Emitted when a committee is finalized (sortition complete, DKG starting).
    /// @param e3Id The ID of the E3.
    event CommitteeFinalized(uint256 indexed e3Id);

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
    //                  Structs                               //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice This struct contains the parameters to submit a request to Enclave.
    /// @param threshold The M/N threshold for the committee.
    /// @param inputWindow When the program will start and stop accepting inputs.
    /// @param e3Program The address of the E3 Program.
    /// @param e3ProgramParams The ABI encoded computation parameters.
    /// @param computeProviderParams The ABI encoded compute provider parameters.
    /// @param customParams Arbitrary ABI-encoded application-defined parameters.
    struct E3RequestParams {
        uint32[2] threshold;
        uint256[2] inputWindow;
        IE3Program e3Program;
        bytes e3ProgramParams;
        bytes computeProviderParams;
        bytes customParams;
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                  Core Entrypoints                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice This function should be called to request a computation within an Encrypted Execution Environment (E3).
    /// @dev This function MUST emit the E3Requested event.
    /// @param requestParams The parameters for the E3 request.
    /// @return e3Id ID of the E3.
    /// @return e3 The E3 struct.
    function request(
        E3RequestParams calldata requestParams
    ) external returns (uint256 e3Id, E3 memory e3);

    /// @notice This function should be called to publish output data for an Encrypted Execution Environment (E3).
    /// @dev This function MUST emit the CiphertextOutputPublished event.
    /// @param e3Id ID of the E3.
    /// @param ciphertextOutput ABI encoded output data to verify.
    /// @param proof ABI encoded data to verify the ciphertextOutput.
    /// @return success True if the output was successfully published.
    function publishCiphertextOutput(
        uint256 e3Id,
        bytes calldata ciphertextOutput,
        bytes calldata proof
    ) external returns (bool success);

    /// @notice This function publishes the plaintext output of an Encrypted Execution Environment (E3).
    /// @dev This function MUST revert if the output has not been published.
    /// @dev This function MUST emit the PlaintextOutputPublished event.
    /// @param e3Id ID of the E3.
    /// @param plaintextOutput ABI encoded plaintext output.
    /// @param proof ABI encoded data to verify the plaintextOutput.
    function publishPlaintextOutput(
        uint256 e3Id,
        bytes calldata plaintextOutput,
        bytes calldata proof
    ) external returns (bool success);

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Set Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice This function should be called to set the maximum duration of requested computations.
    /// @param _maxDuration The maximum duration of a computation in seconds.
    function setMaxDuration(uint256 _maxDuration) external;

    /// @notice Sets the Ciphernode Registry contract address.
    /// @dev This function MUST revert if the address is zero or the same as the current registry.
    /// @param _ciphernodeRegistry The address of the new Ciphernode Registry contract.
    function setCiphernodeRegistry(
        ICiphernodeRegistry _ciphernodeRegistry
    ) external;

    /// @notice Sets the Bonding Registry contract address.
    /// @dev This function MUST revert if the address is zero or the same as the current registry.
    /// @param _bondingRegistry The address of the new Bonding Registry contract.
    function setBondingRegistry(IBondingRegistry _bondingRegistry) external;

    /// @notice Sets the fee token used for E3 payments.
    /// @dev This function MUST revert if the address is zero or the same as the current fee token.
    /// @param _feeToken The address of the new fee token.
    function setFeeToken(IERC20 _feeToken) external;

    /// @notice This function should be called to enable an E3 Program.
    /// @param e3Program The address of the E3 Program.
    function enableE3Program(IE3Program e3Program) external;

    /// @notice This function should be called to disable an E3 Program.
    /// @param e3Program The address of the E3 Program.
    function disableE3Program(IE3Program e3Program) external;

    /// @notice Sets or enables a decryption verifier for a specific encryption scheme.
    /// @dev This function MUST revert if the verifier address is zero or already set to the same value.
    /// @param encryptionSchemeId The unique identifier for the encryption scheme.
    /// @param decryptionVerifier The address of the decryption verifier contract.
    function setDecryptionVerifier(
        bytes32 encryptionSchemeId,
        IDecryptionVerifier decryptionVerifier
    ) external;

    /// @notice Disables a previously enabled encryption scheme.
    /// @dev This function MUST revert if the encryption scheme is not currently enabled.
    /// @param encryptionSchemeId The unique identifier for the encryption scheme to disable.
    function disableEncryptionScheme(bytes32 encryptionSchemeId) external;

    /// @notice Sets the allowed E3 program parameters.
    /// @dev This function enables specific parameter sets for E3 programs (e.g., BFV encryption parameters).
    /// @param _e3ProgramsParams Array of ABI encoded parameter sets to allow.
    function setE3ProgramsParams(bytes[] memory _e3ProgramsParams) external;

    /// @notice Removes previously allowed E3 program parameter sets.
    /// @dev This function revokes specific parameter sets that should no longer be allowed.
    /// @param _e3ProgramsParams Array of ABI encoded parameter sets to remove.
    function removeE3ProgramsParams(bytes[] memory _e3ProgramsParams) external;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Get Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice This function should be called to retrieve the details of an Encrypted Execution Environment (E3).
    /// @dev This function MUST revert if the E3 does not exist.
    /// @param e3Id ID of the E3.
    /// @return e3 The struct representing the requested E3.
    function getE3(uint256 e3Id) external view returns (E3 memory e3);

    /// @notice This function returns the fee of an E3
    /// @dev This function MUST revert if the E3 parameters are invalid.
    /// @param e3Params the struct representing the E3 request parameters
    /// @return fee the fee of the E3
    function getE3Quote(
        E3RequestParams calldata e3Params
    ) external view returns (uint256 fee);

    /// @notice Returns the decryption verifier for a given encryption scheme.
    /// @param encryptionSchemeId The unique identifier for the encryption scheme.
    /// @return The decryption verifier contract for the specified encryption scheme.
    function getDecryptionVerifier(
        bytes32 encryptionSchemeId
    ) external view returns (IDecryptionVerifier);

    /// @notice Returns the ERC20 token used to pay for E3 fees.
    function feeToken() external view returns (IERC20);

    /// @notice Returns the BondingRegistry contract.
    function bondingRegistry() external view returns (IBondingRegistry);

    /// @notice Called by CiphernodeRegistry when committee is finalized (sortition complete).
    /// @dev Updates E3 lifecycle to CommitteeFinalized stage, starts DKG deadline.
    /// @param e3Id ID of the E3.
    function onCommitteeFinalized(uint256 e3Id) external;

    /// @notice Called by CiphernodeRegistry when committee public key is published (DKG complete).
    /// @dev Updates E3 lifecycle to KeyPublished stage.
    /// @param e3Id ID of the E3.
    /// @param committeePublicKeyHash Hash of the committee public key.
    function onCommitteePublished(
        uint256 e3Id,
        bytes32 committeePublicKeyHash
    ) external;

    /// @notice Called by authorized contracts to mark an E3 as failed with a specific reason.
    /// @dev Updates E3 lifecycle to Failed stage with the given reason.
    /// @param e3Id ID of the E3.
    /// @param reason The failure reason from FailureReason enum.
    function onE3Failed(uint256 e3Id, uint8 reason) external;

    /// @notice Escrow slashed funds for deferred distribution
    /// @dev Called by SlashingManager. Proxies to E3RefundManager.
    /// @param e3Id The E3 ID.
    /// @param amount Amount of slashed funds to escrow.
    function escrowSlashedFunds(uint256 e3Id, uint256 amount) external;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                  Lifecycle Functions                   //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice Anyone can mark an E3 as failed if timeout passed
    /// @param e3Id The E3 ID
    /// @return reason The failure reason
    function markE3Failed(uint256 e3Id) external returns (FailureReason reason);

    /// @notice Check if E3 can be marked as failed
    /// @param e3Id The E3 ID
    /// @return canFail Whether failure condition is met
    /// @return reason The failure reason if applicable
    function checkFailureCondition(
        uint256 e3Id
    ) external view returns (bool canFail, FailureReason reason);

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

    /// @notice Get timeout configuration
    /// @return config The current timeout config
    function getTimeoutConfig()
        external
        view
        returns (E3TimeoutConfig memory config);

    /// @notice Set timeout configuration
    /// @param config The new timeout config
    function setTimeoutConfig(E3TimeoutConfig calldata config) external;
}
