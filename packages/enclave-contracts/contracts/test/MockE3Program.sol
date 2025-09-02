// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IE3Program, IInputValidator } from "../interfaces/IE3Program.sol";

contract MockE3Program is IE3Program {
    error invalidParams(bytes e3ProgramParams, bytes computeProviderParams);
    error InvalidInputValidator();
    error E3AlreadyInitialized();
    bytes32 public constant ENCRYPTION_SCHEME_ID = keccak256("fhe.rs:BFV");

    IInputValidator public inputValidator;
    mapping(uint256 e3Id => bytes32 paramsHash) public paramsHashes;

    constructor(IInputValidator _inputValidator) {
        if (address(_inputValidator) == address(0)) {
            revert InvalidInputValidator();
        }

        inputValidator = _inputValidator;
    }

    function validate(
        uint256 e3Id,
        uint256,
        bytes calldata e3ProgramParams,
        bytes calldata computeProviderParams
    ) external returns (bytes32, IInputValidator) {
        require(
            computeProviderParams.length == 32,
            invalidParams(e3ProgramParams, computeProviderParams)
        );

        require(paramsHashes[e3Id] == bytes32(0), E3AlreadyInitialized());
        paramsHashes[e3Id] = keccak256(e3ProgramParams);

        paramsHashes[e3Id] = keccak256(e3ProgramParams);
        return (ENCRYPTION_SCHEME_ID, inputValidator);
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
