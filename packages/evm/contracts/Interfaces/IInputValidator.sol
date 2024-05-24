// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

interface IInputValidator {
    /// @notice This function should be called by the Enclave contract to validate the input parameters.
    /// @param params ABI encoded input parameters.
    function validate(address sender, bytes calldata params) external returns (bool success);
}
