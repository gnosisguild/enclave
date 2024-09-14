// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IInputValidator } from "./IInputValidator.sol";

interface IE3Program {
    /// @notice This function should be called by the Enclave contract to validate the computation parameters.
    /// @param e3Id ID of the E3.
    /// @param seed Seed for the computation.
    /// @param params ABI encoded computation parameters.
    /// @return inputValidator The input validator to be used for the computation.
    function validate(
        uint256 e3Id,
        uint256 seed,
        bytes calldata params
    ) external returns (IInputValidator inputValidator);

    /// @notice This function should be called by the Enclave contract to verify the decrypted output of an E3.
    /// @param e3Id ID of the E3.
    /// @param outputData ABI encoded output data to be verified.
    /// @return output The output data to be published.
    /// @return success Whether the output data is valid.
    function verify(
        uint256 e3Id,
        bytes memory outputData
    ) external returns (bytes memory output, bool success);
}
