// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { IOutputVerifier } from "../interfaces/IOutputVerifier.sol";

contract MockOutputVerifier is IOutputVerifier {
    function verify(
        uint256,
        bytes memory data
    ) external pure returns (bytes memory output, bool success) {
        output = data;

        if (output.length > 0) success = true;
    }
}
