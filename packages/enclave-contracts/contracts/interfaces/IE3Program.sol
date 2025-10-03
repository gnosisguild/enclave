// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IInputValidator } from "./IInputValidator.sol";

/**
 * @title IE3Program
 * @notice Interface for E3 program validation and verification
 * @dev E3 programs define the computation logic and validation rules for encrypted execution environments
 */
interface IE3Program {
    /// @notice Validate E3 computation parameters and return encryption scheme and input validator
    /// @dev This function is called by the Enclave contract during E3 request to configure the computation
    /// @param e3Id ID of the E3 computation
    /// @param seed Random seed for the computation
    /// @param e3ProgramParams ABI encoded E3 program parameters
    /// @param computeProviderParams ABI encoded compute provider parameters
    /// @return encryptionSchemeId ID of the encryption scheme to be used for the computation
    /// @return inputValidator The input validator to be used for the computation
    function validate(
        uint256 e3Id,
        uint256 seed,
        bytes calldata e3ProgramParams,
        bytes calldata computeProviderParams
    )
        external
        returns (bytes32 encryptionSchemeId, IInputValidator inputValidator);

    /// @notice Verify the ciphertext output of an E3 computation
    /// @dev This function is called by the Enclave contract when ciphertext output is published
    /// @param e3Id ID of the E3 computation
    /// @param ciphertextOutputHash The keccak256 hash of output data to be verified
    /// @param proof ABI encoded data to verify the ciphertextOutputHash
    /// @return success Whether the output data is valid
    function verify(
        uint256 e3Id,
        bytes32 ciphertextOutputHash,
        bytes memory proof
    ) external returns (bool success);
}
