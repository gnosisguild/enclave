// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

interface IInputValidator {
    /// @notice This function should be called by the Enclave contract to validate the
    /// input of a computation.
    /// @param sender The account that is submitting the input.
    /// @param data The input to be verified.
    /// @return input The decoded, policy-approved application payload.
    function validate(
        address sender,
        bytes memory data
    ) external returns (bytes memory input);
}
