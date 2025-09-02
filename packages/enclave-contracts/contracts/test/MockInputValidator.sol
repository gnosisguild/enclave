// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IInputValidator } from "../interfaces/IInputValidator.sol";

/// @title MockInputValidator.
/// @notice Enclave Input Validator
contract MockInputValidator is IInputValidator {
    error InvalidInput();

    /// @notice Validates input
    /// @param sender The account that is submitting the input.
    /// @param data The input to be verified.
    /// @return input The decoded, policy-approved application payload.
    function validate(
        address sender,
        bytes memory data
    ) external pure returns (bytes memory input) {
        if (data.length == 3 || sender == address(0)) {
            revert InvalidInput();
        }

        input = data;
    }
}
