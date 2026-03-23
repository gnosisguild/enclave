// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IPkVerifier } from "../interfaces/IPkVerifier.sol";

contract MockPkVerifier is IPkVerifier {
    function verify(
        bytes memory proof,
        bytes memory /* foldProof */
    ) external pure returns (bytes32 pkCommitment) {
        (, bytes32[] memory publicInputs) = abi.decode(
            proof,
            (bytes, bytes32[])
        );
        require(publicInputs.length > 0, "MockPkVerifier: no public inputs");
        return publicInputs[publicInputs.length - 1];
    }
}
