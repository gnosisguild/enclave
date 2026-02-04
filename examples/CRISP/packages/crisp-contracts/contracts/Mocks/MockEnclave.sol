// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { E3 } from "@enclave-e3/contracts/contracts/interfaces/IE3.sol";
import { IE3Program } from "@enclave-e3/contracts/contracts/interfaces/IE3Program.sol";
import { IDecryptionVerifier } from "@enclave-e3/contracts/contracts/interfaces/IDecryptionVerifier.sol";

contract MockEnclave {
  bytes public plaintextOutput;
  bytes32 public committeePublicKey;

  uint256 public nextE3Id;

  mapping(uint256 => E3) public e3s;

  function request(address program) external {
    e3s[nextE3Id] = E3({
      seed: 0,
      threshold: [uint32(1), uint32(2)],
      requestBlock: 0,
      startWindow: [uint256(0), uint256(0)],
      duration: 0,
      expiration: 0,
      encryptionSchemeId: bytes32(0),
      e3Program: IE3Program(program),
      e3ProgramParams: bytes(""),
      customParams: abi.encode(address(0), nextE3Id, 2, 0, 0),
      decryptionVerifier: IDecryptionVerifier(address(0)),
      committeePublicKey: committeePublicKey,
      ciphertextOutput: bytes32(0),
      plaintextOutput: plaintextOutput,
      requester: address(0)
    });

    IE3Program(program).validate(nextE3Id, 0, bytes(""), bytes(""), abi.encode(address(0), nextE3Id, 2));

    nextE3Id++;
  }

  function setPlaintextOutput(bytes memory plaintext) external {
    plaintextOutput = plaintext;
  }

  function setCommitteePublicKey(bytes32 publicKeyHash) external {
    committeePublicKey = publicKeyHash;
  }

  function getE3(uint256) external view returns (E3 memory) {
    return
      E3({
        seed: 0,
        threshold: [uint32(1), uint32(2)],
        requestBlock: 0,
        startWindow: [uint256(0), uint256(0)],
        duration: 0,
        expiration: 0,
        encryptionSchemeId: bytes32(0),
        e3Program: IE3Program(address(0)),
        e3ProgramParams: bytes(""),
        customParams: abi.encode(address(0), 0, 2, 0, 0),
        decryptionVerifier: IDecryptionVerifier(address(0)),
        committeePublicKey: committeePublicKey,
        ciphertextOutput: bytes32(0),
        plaintextOutput: plaintextOutput,
        requester: address(0)
      });
  }
}
