// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IPkVerifier } from "../../interfaces/IPkVerifier.sol";
import { ICircuitVerifier } from "../../interfaces/ICircuitVerifier.sol";

/**
 * @title BfvPkVerifier
 * @notice Verifies the DkgAggregator (EVM) proof produced by the recursive
 *         aggregation pipeline (node folds + C5/pk_aggregation verified
 *         internally). Binds the proof to a caller-supplied `pkCommitment`.
 * @dev Used when the Enclave is configured with encryptionSchemeId
 *      keccak256("fhe.rs:BFV"). The aggregator circuit's last public input is
 *      the Safe-based aggregated PK commitment.
 */
contract BfvPkVerifier is IPkVerifier {
    /// @notice Underlying Honk verifier for the DkgAggregator circuit.
    ICircuitVerifier public immutable circuitVerifier;

    constructor(address _circuitVerifier) {
        circuitVerifier = ICircuitVerifier(_circuitVerifier);
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

        if (publicInputs.length == 0) {
            return false;
        }
        if (publicInputs[publicInputs.length - 1] != pkCommitment) {
            return false;
        }
        return circuitVerifier.verify(rawProof, publicInputs);
    }
}
