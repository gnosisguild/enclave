// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

interface IComputationModule {
    /// @notice This function should be called by the Enclave contract to validate the computation parameters.
    /// @param params ABI encoded computation parameters.
    function validate(bytes calldata params) external returns (address inputVerifier);
}
