// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { IOutputVerifier } from "./IOutputVerifier.sol";

interface IComputeProvider {
    /// @notice This function should be called by the Enclave contract to validate the compute provider parameters.
    /// @param params ABI encoded compute provider parameters.
    function validate(
        bytes calldata params
    ) external returns (IOutputVerifier outputVerifier);
}
