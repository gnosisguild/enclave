// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity 0.8.28;

import { IE3Program } from "../interfaces/IE3Program.sol";
import { IEnclave } from "../interfaces/IEnclave.sol";

contract MockE3Program is IE3Program {
    error InvalidParams(bytes e3ProgramParams, bytes computeProviderParams);
    error E3AlreadyInitialized();
    error InvalidInput();

    bytes32 public constant ENCRYPTION_SCHEME_ID = keccak256("fhe.rs:BFV");

    /// @notice Optional Enclave contract — when set, `publishInput` forwards
    /// `data` to `enclave.publishCiphertextOutput`, which is what the integration
    /// tests rely on to trigger the ciphernode decryption pipeline. A real E3
    /// program would aggregate user inputs off-chain into a single ciphertext;
    /// the mock short-circuits that step by treating the input as the ciphertext.
    IEnclave public enclave;

    mapping(uint256 e3Id => bytes32 paramsHash) public paramsHashes;

    function setEnclave(IEnclave _enclave) external {
        enclave = _enclave;
    }

    function validate(
        uint256 e3Id,
        uint256,
        bytes calldata e3ProgramParams,
        bytes calldata computeProviderParams,
        bytes calldata
    ) external returns (bytes32) {
        require(
            computeProviderParams.length == 32,
            InvalidParams(e3ProgramParams, computeProviderParams)
        );

        require(paramsHashes[e3Id] == bytes32(0), E3AlreadyInitialized());
        paramsHashes[e3Id] = keccak256(e3ProgramParams);
        return ENCRYPTION_SCHEME_ID;
    }

    function publishInput(uint256 e3Id, bytes memory data) external {
        if (data.length == 3) {
            revert InvalidInput();
        }
        if (address(enclave) != address(0)) {
            // Test-only: external call to Enclave with no reentrancy guard.
            // Deliberate — this contract is only deployed in integration tests
            // and `enclave` is set via `setEnclave` to the trusted Enclave
            // proxy. Do not copy this pattern into a production E3 program.
            // Pass `data` as the proof too so `MockE3Program.verify` (which
            // requires `proof.length > 0`) returns true.
            enclave.publishCiphertextOutput(e3Id, data, data);
        }
    }

    function verify(
        uint256,
        bytes32,
        bytes memory data
    ) external pure returns (bool success) {
        // data parameter available for custom validation logic
        if (data.length > 0) success = true;
    }
}
