// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IDecryptionVerifier } from "../interfaces/IDecryptionVerifier.sol";

contract MockDecryptionVerifier is IDecryptionVerifier {
    /// @dev Test-only: proofs whose first 4 bytes are `0xdeadbeef` return false so
    ///      tests can exercise `InvalidOutput` when proof aggregation is on.
    bytes4 private constant _FAIL_MAGIC = 0xdeadbeef;

    function verify(
        bytes32,
        bytes calldata proof
    ) external pure returns (bool success) {
        if (proof.length >= 4 && bytes4(proof[0:4]) == _FAIL_MAGIC) {
            return false;
        }
        success = proof.length > 0;
    }
}
