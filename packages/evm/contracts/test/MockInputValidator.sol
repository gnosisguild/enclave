// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { IInputValidator } from "../interfaces/IInputValidator.sol";

contract MockInputValidator is IInputValidator {
    function validate(
        address,
        bytes memory params
    ) external pure returns (bytes memory input, bool success) {
        input = abi.decode(params, (bytes));

        if (input.length == 3) {
            success = false;
        } else {
            success = true;
        }
    }
}
