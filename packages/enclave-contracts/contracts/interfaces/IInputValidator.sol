// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

/**
 * @title IInputValidator
 * @notice Interface for validating computation inputs
 * @dev Input validators enforce access control and validation rules for E3 computation inputs
 */
interface IInputValidator {
    /// @notice Validate and process input data for a computation
    /// @dev This function is called by the Enclave contract when input is published
    /// @param sender The account that is submitting the input
    /// @param data The input data to be validated
    /// @return input The decoded, policy-approved application payload
    function validate(
        address sender,
        bytes memory data
    ) external returns (bytes memory input);
}
