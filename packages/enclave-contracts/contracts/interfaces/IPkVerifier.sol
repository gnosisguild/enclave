// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

/**
 * @title IPkVerifier
 * @notice Interface for the DkgAggregator (EVM) proof verifier.
 * @dev The DkgAggregator circuit internally verifies the node-fold and C5
 *      (pk_aggregation) sub-proofs; this on-chain verifier only needs to
 *      verify the final EVM proof and enforce that its last public input
 *      matches the committee's aggregated public-key commitment.
 */
interface IPkVerifier {
    /// @notice Verify a DkgAggregator EVM proof and bind it to `pkCommitment`.
    /// @param pkCommitment Hash-based aggregated PK commitment the proof must attest to
    ///        (equals `publicInputs[publicInputs.length - 1]`).
    /// @param proof ABI-encoded `(bytes rawProof, bytes32[] publicInputs)`.
    /// @return success True if the proof is valid and its last public input equals `pkCommitment`.
    function verify(
        bytes32 pkCommitment,
        bytes calldata proof
    ) external view returns (bool success);
}
