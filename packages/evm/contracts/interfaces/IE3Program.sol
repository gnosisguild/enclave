// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IEnclavePolicy } from "./IEnclavePolicy.sol";

interface IE3Program {
    /// @notice This function should be called by the Enclave contract to validate the computation parameters.
    /// @param e3Id ID of the E3.
    /// @param seed Seed for the computation.
    /// @param e3ProgramParams ABI encoded computation parameters.
    /// @param computeProviderParams ABI encoded compute provider parameters.
    /// @return encryptionSchemeId ID of the encryption scheme to be used for the computation.
    /// @return inputValidator The input validator to be used for the computation.
    function validate(
        uint256 e3Id,
        uint256 seed,
        bytes calldata e3ProgramParams,
        bytes calldata computeProviderParams
    )
        external
        returns (bytes32 encryptionSchemeId, IEnclavePolicy inputValidator);

    /// @notice This function should be called by the Enclave contract to verify the decrypted output of an E3.
    /// @param e3Id ID of the E3.
    /// @param ciphertextOutputHash The keccak256 hash of output data to be verified.
    /// @param proof ABI encoded data to verify the ciphertextOutputHash.
    /// @return success Whether the output data is valid.
    function verify(
        uint256 e3Id,
        bytes32 ciphertextOutputHash,
        bytes memory proof
    ) external returns (bool success);
}
