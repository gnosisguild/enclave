// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { IE3Program, IInputValidator } from "../interfaces/IE3Program.sol";

contract MockE3Program is IE3Program {
    error invalidParams(bytes params);

    function validate(
        bytes memory params
    ) external pure returns (IInputValidator inputValidator) {
        require(params.length == 32, "invalid params");
        // solhint-disable no-inline-assembly
        assembly {
            inputValidator := mload(add(params, 32))
        }
    }

    function verify(
        uint256,
        bytes memory data
    ) external pure returns (bytes memory output, bool success) {
        output = data;
        if (output.length > 0) success = true;
    }
}
