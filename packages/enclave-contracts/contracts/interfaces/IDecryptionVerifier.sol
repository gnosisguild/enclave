// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

/**
 * @title IDecryptionVerifier
 * @notice Interface for the DecryptionAggregator (EVM) proof verifier.
 * @dev The DecryptionAggregator circuit internally verifies the C6-fold and C7
 *      (decrypted_shares_aggregation) sub-proofs; this on-chain verifier only
 *      needs to verify the final EVM proof and bind it to the claimed plaintext.
 */
interface IDecryptionVerifier {
    /// @notice Verify a DecryptionAggregator EVM proof and bind it to `plaintextOutputHash`.
    /// @param plaintextOutputHash `keccak256(plaintextOutput)` expected by the Enclave.
    /// @param proof ABI-encoded `(bytes rawProof, bytes32[] publicInputs)`.
    /// @return success True if the proof is valid and its embedded plaintext matches `plaintextOutputHash`.
    function verify(
        bytes32 plaintextOutputHash,
        bytes calldata proof
    ) external view returns (bool success);
}
