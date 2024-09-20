// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {
    IE3Program,
    IInputValidator,
    IDecryptionVerifier
} from "../interfaces/IE3Program.sol";

contract MockE3Program is IE3Program {
    error invalidParams(bytes e3ProgramParams, bytes computeProviderParams);

    function validate(
        uint256,
        uint256,
        bytes memory e3ProgramParams,
        bytes memory computeProviderParams
    )
        external
        pure
        returns (
            bytes32 encryptionSchemeId,
            IInputValidator inputValidator,
            IDecryptionVerifier decryptionVerifier
        )
    {
        require(
            e3ProgramParams.length == 32 && computeProviderParams.length == 32,
            invalidParams(e3ProgramParams, computeProviderParams)
        );
        // solhint-disable no-inline-assembly
        assembly {
            inputValidator := mload(add(e3ProgramParams, 32))
            decryptionVerifier := mload(add(computeProviderParams, 32))
        }
        encryptionSchemeId = 0x0000000000000000000000000000000000000000000000000000000000000001;
    }

    function verify(
        uint256,
        bytes32,
        bytes memory data
    ) external pure returns (bool success) {
        data;
        if (data.length > 0) success = true;
    }
}
