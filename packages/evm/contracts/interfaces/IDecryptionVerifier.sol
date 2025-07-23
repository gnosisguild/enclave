// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

interface IDecryptionVerifier {
    /// @notice This function should be called by the Enclave contract to verify the
    /// decryption of output of a computation.
    /// @param e3Id ID of the E3.
    /// @param plaintextOutputHash The keccak256 hash of the plaintext output to be verified.
    /// @param proof ABI encoded proof of the given output hash.
    /// @return success Whether or not the plaintextOutputHash was successfully verified.
    function verify(
        uint256 e3Id,
        bytes32 plaintextOutputHash,
        bytes memory proof
    ) external view returns (bool success);
}
