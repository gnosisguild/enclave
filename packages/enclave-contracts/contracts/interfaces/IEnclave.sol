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
    //                         Events                         //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice This event MUST be emitted when an Encrypted Execution Environment (E3) is successfully requested.
    /// @param e3Id ID of the E3.
    /// @param e3 Details of the E3.
    /// @param e3Program Address of the Computation module selected.
    event E3Requested(uint256 e3Id, E3 e3, IE3Program indexed e3Program);

    /// @notice This event MUST be emitted when an Encrypted Execution Environment (E3) is successfully activated.
    /// @param e3Id ID of the E3.
    /// @param expiration Timestamp when committee duties expire.
    /// @param committeePublicKey Public key of the committee.
    event E3Activated(
        uint256 e3Id,
        uint256 expiration,
        bytes committeePublicKey
    );

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

    /// @notice This event MUST be emitted any time the `sortitionSubmissionWindow` is set.
    /// @param sortitionSubmissionWindow The submission window for the E3 sortition in seconds.
    event SortitionSubmissionWindowSet(uint256 sortitionSubmissionWindow);

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

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                  Structs                               //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @notice This struct contains the parameters to submit a request to Enclave.
    /// @param threshold The M/N threshold for the committee.
    /// @param startWindow The start window for the computation.
    /// @param duration The duration of the computation in seconds.
    /// @param e3Program The address of the E3 Program.
    /// @param e3ProgramParams The ABI encoded computation parameters.
    /// @param computeProviderParams The ABI encoded compute provider parameters.
    /// @param customParams Arbitrary ABI-encoded application-defined parameters.
    struct E3RequestParams {
        uint32[2] threshold;
        uint256[2] startWindow;
        uint256 duration;
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

    /// @notice This function should be called to activate an Encrypted Execution Environment (E3) once it has been
    /// initialized and is ready for input.
    /// @dev This function MUST emit the E3Activated event.
    /// @dev This function MUST revert if the given E3 has not yet been requested.
    /// @dev This function MUST revert if the selected node committee has not yet published a public key.
    /// @param e3Id ID of the E3.
    /// @param publicKey Public key of the committee.
    /// @return success True if the E3 was successfully activated.
    function activate(
        uint256 e3Id,
        bytes calldata publicKey
    ) external returns (bool success);

    /// @notice This function should be called to publish input data for Encrypted Execution Environment (E3).
    /// @dev This function MUST revert if the E3 is not yet activated.
    /// @dev This function MUST emit the InputPublished event.
    /// @param e3Id ID of the E3.
    /// @param data ABI encoded input data to publish.
    /// @return success True if the input was successfully published.
    function publishInput(
        uint256 e3Id,
        bytes calldata data
    ) external returns (bool success);

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
    /// @return success True if the max duration was successfully set.
    function setMaxDuration(
        uint256 _maxDuration
    ) external returns (bool success);

    /// @notice This function should be called to set the submission window for the E3 sortition.
    /// @param _sortitionSubmissionWindow The submission window for the E3 sortition in seconds.
    /// @return success True if the sortition submission window was successfully set.
    function setSortitionSubmissionWindow(
        uint256 _sortitionSubmissionWindow
    ) external returns (bool success);

    /// @notice Sets the Ciphernode Registry contract address.
    /// @dev This function MUST revert if the address is zero or the same as the current registry.
    /// @param _ciphernodeRegistry The address of the new Ciphernode Registry contract.
    /// @return success True if the registry was successfully set.
    function setCiphernodeRegistry(
        ICiphernodeRegistry _ciphernodeRegistry
    ) external returns (bool success);

    /// @notice Sets the Bonding Registry contract address.
    /// @dev This function MUST revert if the address is zero or the same as the current registry.
    /// @param _bondingRegistry The address of the new Bonding Registry contract.
    /// @return success True if the registry was successfully set.
    function setBondingRegistry(
        IBondingRegistry _bondingRegistry
    ) external returns (bool success);

    /// @notice Sets the fee token used for E3 payments.
    /// @dev This function MUST revert if the address is zero or the same as the current fee token.
    /// @param _feeToken The address of the new fee token.
    /// @return success True if the fee token was successfully set.
    function setFeeToken(IERC20 _feeToken) external returns (bool success);

    /// @notice This function should be called to enable an E3 Program.
    /// @param e3Program The address of the E3 Program.
    /// @return success True if the E3 Program was successfully enabled.
    function enableE3Program(
        IE3Program e3Program
    ) external returns (bool success);

    /// @notice This function should be called to disable an E3 Program.
    /// @param e3Program The address of the E3 Program.
    /// @return success True if the E3 Program was successfully disabled.
    function disableE3Program(
        IE3Program e3Program
    ) external returns (bool success);

    /// @notice Sets or enables a decryption verifier for a specific encryption scheme.
    /// @dev This function MUST revert if the verifier address is zero or already set to the same value.
    /// @param encryptionSchemeId The unique identifier for the encryption scheme.
    /// @param decryptionVerifier The address of the decryption verifier contract.
    /// @return success True if the verifier was successfully set.
    function setDecryptionVerifier(
        bytes32 encryptionSchemeId,
        IDecryptionVerifier decryptionVerifier
    ) external returns (bool success);

    /// @notice Disables a previously enabled encryption scheme.
    /// @dev This function MUST revert if the encryption scheme is not currently enabled.
    /// @param encryptionSchemeId The unique identifier for the encryption scheme to disable.
    /// @return success True if the encryption scheme was successfully disabled.
    function disableEncryptionScheme(
        bytes32 encryptionSchemeId
    ) external returns (bool success);

    /// @notice Sets the allowed E3 program parameters.
    /// @dev This function enables specific parameter sets for E3 programs (e.g., BFV encryption parameters).
    /// @param _e3ProgramsParams Array of ABI encoded parameter sets to allow.
    /// @return success True if the parameters were successfully set.
    function setE3ProgramsParams(
        bytes[] memory _e3ProgramsParams
    ) external returns (bool success);

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

    /// @notice This function returns root of the input merkle tree for a given E3.
    /// @dev This function MUST revert if the E3 does not exist.
    /// @param e3Id ID of the E3.
    /// @return root The root of the input merkle tree.
    function getInputRoot(uint256 e3Id) external view returns (uint256 root);

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
}
