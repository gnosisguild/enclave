// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

/**
 * @title IDecryptionVerifier
 * @notice Interface for verifying decrypted computation outputs
 * @dev Implements cryptographic verification of plaintext outputs from encrypted computations
 */
interface IDecryptionVerifier {
    /// @notice Verify the decryption of a computation output
    /// @dev This function is called by the Enclave contract when plaintext output is published
    /// @param e3Id ID of the E3 computation
    /// @param plaintextOutputHash The keccak256 hash of the plaintext output to be verified
    /// @param proof ABI encoded proof of the decryption validity
    /// @return success Whether the plaintextOutputHash was successfully verified
    function verify(
        uint256 e3Id,
        bytes32 plaintextOutputHash,
        bytes memory proof
    ) external view returns (bool success);
}
