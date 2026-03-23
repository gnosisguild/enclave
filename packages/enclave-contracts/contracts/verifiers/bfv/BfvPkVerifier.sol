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
 * @notice Verifies C5 (pk_aggregation) proofs on-chain. Delegates to the Honk
 *         ThresholdPkAggregationVerifier and returns the aggregate commitment from public inputs.
 *         Optional `foldProof` is ABI-encoded (bytes, bytes32[]) for RecursiveAggregationFoldVerifier
 *         (DKG cross-node fold); pass empty bytes to skip.
 * @dev Use with encryptionSchemeId keccak256("fhe.rs:BFV"). Commitment count is enforced by the VK.
 */
contract BfvPkVerifier is IPkVerifier {
    ICircuitVerifier public immutable circuitVerifier;
    ICircuitVerifier public immutable foldVerifier;

    constructor(address _circuitVerifier, address _foldVerifier) {
        circuitVerifier = ICircuitVerifier(_circuitVerifier);
        foldVerifier = ICircuitVerifier(_foldVerifier);
    }

    /// @inheritdoc IPkVerifier
    function verify(
        bytes memory proof,
        bytes memory foldProof
    ) external view override returns (bytes32 pkCommitment) {
        (bytes memory rawProof, bytes32[] memory publicInputs) = abi.decode(
            proof,
            (bytes, bytes32[])
        );

        require(publicInputs.length > 0, "BfvPkVerifier: no public inputs");
        require(
            circuitVerifier.verify(rawProof, publicInputs),
            "BfvPkVerifier: invalid proof"
        );

        _verifyFold(foldProof);

        return publicInputs[publicInputs.length - 1];
    }

    function _verifyFold(bytes memory foldProof) internal view {
        if (foldProof.length == 0) {
            return;
        }

        (bytes memory foldRawProof, bytes32[] memory foldPublicInputs) = abi
            .decode(foldProof, (bytes, bytes32[]));

        require(
            foldVerifier.verify(foldRawProof, foldPublicInputs),
            "BfvPkVerifier: invalid fold proof"
        );
    }
}
