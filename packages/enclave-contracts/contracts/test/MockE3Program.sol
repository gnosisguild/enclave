// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IE3Program } from "../interfaces/IE3Program.sol";

contract MockE3Program is IE3Program {
    error InvalidParams(bytes e3ProgramParams, bytes computeProviderParams);
    error E3AlreadyInitialized();

    bytes32 public constant ENCRYPTION_SCHEME_ID = keccak256("fhe.rs:BFV");

    mapping(uint256 e3Id => bytes32 paramsHash) public paramsHashes;

    error InvalidInput();

    function validate(
        uint256 e3Id,
        uint256,
        bytes calldata e3ProgramParams,
        bytes calldata computeProviderParams
    ) external returns (bytes32) {
        require(
            computeProviderParams.length == 32,
            InvalidParams(e3ProgramParams, computeProviderParams)
        );

        require(paramsHashes[e3Id] == bytes32(0), E3AlreadyInitialized());
        paramsHashes[e3Id] = keccak256(e3ProgramParams);
        return ENCRYPTION_SCHEME_ID;
    }

    function validateInput(
        address sender,
        bytes memory data
    ) external pure returns (bytes memory input) {
        if (data.length == 3 || sender == address(0)) {
            revert InvalidInput();
        }

        input = data;
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
