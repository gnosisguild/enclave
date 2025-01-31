// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

interface IInputValidator {
    /// @notice This function should be called by the Enclave contract to validate the input parameters.
    /// @param params ABI encoded input parameters.
    /// @return input The input data to be published.
    /// @return success Whether the input parameters are valid.
    function validate(
        address sender,
        bytes memory params
    ) external view returns (bytes memory input, bool success);
}
