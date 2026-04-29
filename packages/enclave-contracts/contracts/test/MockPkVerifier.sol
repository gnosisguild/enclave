// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IPkVerifier } from "../interfaces/IPkVerifier.sol";

contract MockPkVerifier is IPkVerifier {
    function verify(
        bytes32 pkCommitment,
        bytes calldata proof
    ) external pure returns (bool) {
        (, bytes32[] memory publicInputs) = abi.decode(
            proof,
            (bytes, bytes32[])
        );
        if (publicInputs.length == 0) return false;
        return publicInputs[publicInputs.length - 1] == pkCommitment;
    }
}
