// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import {E3} from "@enclave-e3/contracts/contracts/interfaces/IE3.sol";
import {IE3Program} from "@enclave-e3/contracts/contracts/interfaces/IE3Program.sol";
import {IDecryptionVerifier} from "@enclave-e3/contracts/contracts/interfaces/IDecryptionVerifier.sol";

contract MockEnclave {
    bytes public plaintextOutput;

    function setPlaintextOutput(uint256[] memory plaintext) external {
        plaintextOutput = abi.encode(plaintext);
    }

    function getE3(uint256 e3Id) external view returns (E3 memory) {
        return E3({
            seed: 0,
            threshold: [uint32(1), uint32(2)],
            requestBlock: 0,
            startWindow: [uint256(0), uint256(0)],
            duration: 0,
            expiration: 0,
            encryptionSchemeId: bytes32(0),
            e3Program: IE3Program(address(0)),
            e3ProgramParams: bytes(""),
            customParams: bytes(""),
            decryptionVerifier: IDecryptionVerifier(address(0)),
            committeePublicKey: bytes32(0),
            ciphertextOutput: bytes32(0),
            plaintextOutput: plaintextOutput
        });
    }
}
