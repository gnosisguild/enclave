// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import { IE3Program, IInputValidator } from "../interfaces/IE3Program.sol";

contract MockE3Program is IE3Program {
    error invalidParams(bytes e3ProgramParams, bytes computeProviderParams);

    IInputValidator storageInputValidator;

    constructor(IInputValidator _inputValidator) {
        storageInputValidator = _inputValidator;
    }

    function setInputValidator(IInputValidator _inputValidator) external {
        storageInputValidator = _inputValidator;
    }

    function validate(
        uint256,
        uint256,
        bytes memory e3ProgramParams,
        bytes memory computeProviderParams
    )
        external
        view
        returns (bytes32 encryptionSchemeId, IInputValidator inputValidator)
    {
        require(
            computeProviderParams.length == 32,
            invalidParams(e3ProgramParams, computeProviderParams)
        );
        inputValidator = storageInputValidator;
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
