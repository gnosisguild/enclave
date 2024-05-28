// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { E3, IComputationModule, IExecutionModule } from "./IE3.sol";

interface IEnclave {
    /// @notice This event MUST be emitted when an Encrypted Execution Environment (E3) is successfully requested.
    /// @param e3Id ID of the E3.
    /// @param e3 Details of the E3.
    /// @param poolId ID of the pool of nodes from which the Cypher Node committee was selected.
    /// @param computationModule Address of the Computation module selected.
    /// @param executionModule  Address of the execution module selected.
    event E3Requested(
        uint256 e3Id,
        E3 e3,
        uint256 indexed poolId,
        IComputationModule indexed computationModule,
        IExecutionModule indexed executionModule
    );

    /// @notice This event MUST be emitted when an Encrypted Execution Environment (E3) is successfully activated.
    /// @param e3Id ID of the E3.
    /// @param expiration Timestamp when committee duties expire.
    /// @param committeePublicKey Public key of the committee.
    event E3Activated(uint256 e3Id, uint256 expiration, bytes committeePublicKey);

    /// @notice This event MUST be emitted when an input to an Encrypted Execution Environment (E3) is successfully published.
    /// @param e3Id ID of the E3.
    /// @param data ABI encoded input data.
    event InputPublished(uint256 indexed e3Id, bytes data);

    /// @notice This event MUST be emitted when the plaintext output of an Encrypted Execution Environment (E3) is successfully published.
    /// @param e3Id ID of the E3.
    /// @param plaintextOutput ABI encoded plaintext output.
    event PlaintextOutputPublished(uint256 indexed e3Id, bytes plaintextOutput);

    /// @notice This event MUST be emitted when the ciphertext output of an Encrypted Execution Environment (E3) is successfully published.
    /// @param e3Id ID of the E3.
    /// @param ciphertextOutput ABI encoded ciphertext output.
    event CiphertextOutputPublished(uint256 indexed e3Id, bytes ciphertextOutput);

    /// @notice This function should be called to request a computation within an Encrypted Execution Environment (E3).
    /// @dev This function MUST emit the E3Requested event.
    /// @param poolId ID of the pool of nodes from which to select the committee.
    /// @param threshold The M/N threshold for the committee.
    /// @param duration The duration of the computation in seconds.
    /// @param computationModule Address of the computation module.
    /// @param computationParams ABI encoded computation parameters.
    /// @param executionModule Address of the execution module.
    /// @param emParams ABI encoded execution module parameters.
    /// @return e3Id ID of the E3.
    /// @return e3 The E3 struct.
    function request(
        uint256 poolId,
        uint32[2] calldata threshold,
        uint256 duration,
        IComputationModule computationModule,
        bytes memory computationParams,
        IExecutionModule executionModule,
        bytes memory emParams
    ) external payable returns (uint256 e3Id, E3 memory e3);

    /// @notice This function should be called to activate an Encrypted Execution Environment (E3) once it has been initialized and is ready for input.
    /// @dev This function MUST emit the E3Activated event.
    /// @dev This function MUST revert if the given E3 has not yet been requested.
    /// @dev This function MUST revert if the selected node committee has not yet published a public key.
    /// @param e3Id ID of the E3.
    function activate(uint256 e3Id) external returns (bool success);

    /// @notice This function should be called to publish input data for Encrypted Execution Environment (E3).
    /// @dev This function MUST revert if the E3 is not yet activated.
    /// @dev This function MUST emit the InputPublished event.
    /// @param e3Id ID of the E3.
    /// @param data ABI encoded input data to publish.
    /// @return success True if the input was successfully published.
    function publishInput(uint256 e3Id, bytes calldata data) external returns (bool success);

    /// @notice This function should be called to publish output data for an Encrypted Execution Environment (E3).
    /// @dev This function MUST emit the CiphertextOutputPublished event.
    /// @param e3Id ID of the E3.
    /// @param data ABI encoded output data to verify.
    /// @return success True if the output was successfully published.
    function publishOutput(uint256 e3Id, bytes memory data) external returns (bool success);

    /// @notice This function should be called to decrypt the output of an Encrypted Execution Environment (E3).
    /// @dev This function MUST revert if the output has not been published.
    /// @dev This function MUST emit the PlaintextOutputPublished event.
    /// @param e3Id ID of the E3.
    /// @param data ABI encoded output data to decrypt.
    /// @return success True if the output was successfully decrypted.
    function publishDecryptedOutput(uint256 e3Id, bytes memory data) external returns (bool success);

    /// @notice This function should be called to retrieve the details of an Encrypted Execution Environment (E3).
    /// @dev This function MUST revert if the E3 does not exist.
    /// @param e3Id ID of the E3.
    /// @return e3 The struct representing the requested E3.
    function getE3(uint256 e3Id) external view returns (E3 memory e3);
}
