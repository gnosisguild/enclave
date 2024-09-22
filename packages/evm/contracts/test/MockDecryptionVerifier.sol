// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IDecryptionVerifier } from "../interfaces/IDecryptionVerifier.sol";

contract MockDecryptionVerifier is IDecryptionVerifier {
    function verify(
        uint256,
        bytes32,
        bytes memory data
    ) external pure returns (bool success) {
        data;

        if (data.length > 0) success = true;
    }
}
