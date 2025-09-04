// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import {IInputValidator} from "@enclave-e3/contracts/contracts/interfaces/IInputValidator.sol";

contract InputValidator is IInputValidator {
    error EmptyInputData();

    /// @notice Validates input
    /// @param sender The account that is submitting the input.
    /// @param data The input to be verified.
    /// @return input The input data.
    function validate(
        address sender,
        bytes memory data
    ) external returns (bytes memory input) {
        if (data.length == 0) revert EmptyInputData();

        // You can add your own validation logic here.
        // EXAMPLE: https://github.com/gnosisguild/enclave/blob/main/examples/CRISP/contracts/CRISPInputValidator.sol

        input = data;
    }
}
