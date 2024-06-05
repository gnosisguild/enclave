// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { IOutputVerifier } from "./IOutputVerifier.sol";

interface IExecutionModule {
    /// @notice This function should be called by the Enclave contract to validate the execution module parameters.
    /// @param params ABI encoded execution module parameters.
    function validate(bytes calldata params) external returns (IOutputVerifier outputVerifier);
}
