// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { ISlashVerifier } from "../interfaces/ISlashVerifier.sol";

contract MockSlashingVerifier is ISlashVerifier {
    function verify(
        uint256,
        bytes memory data
    ) external pure returns (bool success) {
        data;

        if (data.length > 0) success = true;
    }
}
