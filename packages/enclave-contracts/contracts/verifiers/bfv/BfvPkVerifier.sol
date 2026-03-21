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
 * @dev Use with encryptionSchemeId keccak256("fhe.rs:BFV"). Commitment count is enforced by the VK.
 */
contract BfvPkVerifier is IPkVerifier {
    ICircuitVerifier public immutable circuitVerifier;

    constructor(address _circuitVerifier) {
        circuitVerifier = ICircuitVerifier(_circuitVerifier);
    }

    /// @inheritdoc IPkVerifier
    function verify(
        bytes memory proof
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

        return publicInputs[publicInputs.length - 1];
    }
}
