// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

interface IOutputVerifier {
    /// @notice This function should be called by the Enclave contract to verify the output of a computation.
    /// @param e3Id ID of the E3.
    /// @param data ABI encoded output data to be verified.
    /// @return output Ciphertext output of the given computation.
    function verify(
        uint256 e3Id,
        bytes memory data
    ) external view returns (bytes memory output, bool success);
}
