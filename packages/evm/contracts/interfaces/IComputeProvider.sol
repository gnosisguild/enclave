// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IDecryptionVerifier } from "./IDecryptionVerifier.sol";

interface IComputeProvider {
    /// @notice This function should be called by the Enclave contract to validate the compute provider parameters.
    /// @param params ABI encoded compute provider parameters.
    function validate(
        uint256 e3Id,
        uint256 seed,
        bytes calldata params
    ) external returns (IDecryptionVerifier decryptionVerifier);
}
