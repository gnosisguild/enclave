// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

/**
 * @title IPkVerifier
 * @notice Interface for C5 (pk_aggregation) proof verification
 * @dev Verifies that the aggregated committee public key was correctly reconstructed from party shares
 */
interface IPkVerifier {
    /// @notice Verify a C5 (pk_aggregation) proof and return the aggregate commitment.
    /// @param proof ABI-encoded (bytes rawProof, bytes32[] publicInputs).
    /// @param foldProof ABI-encoded fold proof (bytes, bytes32[]) or empty to skip.
    /// @return pkCommitment The aggregate public key commitment (last public input).
    /// @dev Reverts if the proof is invalid.
    function verify(
        bytes memory proof,
        bytes memory foldProof
    ) external view returns (bytes32 pkCommitment);
}
