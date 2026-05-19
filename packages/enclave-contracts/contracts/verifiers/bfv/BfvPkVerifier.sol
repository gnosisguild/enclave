// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity 0.8.28;

import { IPkVerifier } from "../../interfaces/IPkVerifier.sol";
import { ICircuitVerifier } from "../../interfaces/ICircuitVerifier.sol";

/**
 * @title BfvPkVerifier
 * @notice Verifies the DkgAggregator (EVM) proof produced by the recursive
 *         aggregation pipeline (node folds + C5/pk_aggregation verified
 *         internally). Binds the proof to a caller-supplied `pkCommitment`.
 * @dev Used when the Enclave is configured with encryptionSchemeId
 *      keccak256("fhe.rs:BFV"). The aggregator circuit's last public input is
 *      the hash-based aggregated PK commitment.
 */
contract BfvPkVerifier is IPkVerifier {
    /// @notice Underlying Honk verifier for the DkgAggregator circuit.
    ICircuitVerifier public immutable circuitVerifier;

    /// @notice Expected recursive VK hash for the nodes_fold sub-circuit (`publicInputs[0]`).
    bytes32 public immutable expectedNodesFoldKeyHash;

    /// @notice Expected recursive VK hash for the C5/pk_aggregation sub-circuit (`publicInputs[1]`).
    bytes32 public immutable expectedC5KeyHash;

    constructor(
        address _circuitVerifier,
        bytes32 _expectedNodesFoldKeyHash,
        bytes32 _expectedC5KeyHash
    ) {
        circuitVerifier = ICircuitVerifier(_circuitVerifier);
        expectedNodesFoldKeyHash = _expectedNodesFoldKeyHash;
        expectedC5KeyHash = _expectedC5KeyHash;
    }

    /// @inheritdoc IPkVerifier
    function verify(
        bytes32 pkCommitment,
        bytes calldata proof
    ) external view override returns (bool) {
        (bytes memory rawProof, bytes32[] memory publicInputs) = abi.decode(
            proof,
            (bytes, bytes32[])
        );

        if (publicInputs.length < 3) {
            return false;
        }
        if (publicInputs[0] != expectedNodesFoldKeyHash) {
            return false;
        }
        if (publicInputs[1] != expectedC5KeyHash) {
            return false;
        }
        if (publicInputs[publicInputs.length - 1] != pkCommitment) {
            return false;
        }
        return circuitVerifier.verify(rawProof, publicInputs);
    }
}
