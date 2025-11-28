// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IRiscZeroVerifier } from '@risc0/ethereum/contracts/IRiscZeroVerifier.sol';
import { IE3Program } from '@enclave-e3/contracts/contracts/interfaces/IE3Program.sol';
import { IEnclave } from '@enclave-e3/contracts/contracts/interfaces/IEnclave.sol';
import { Ownable } from '@openzeppelin/contracts/access/Ownable.sol';
import { LazyIMTData, InternalLazyIMT, PoseidonT3 } from '@zk-kit/lazy-imt.sol/InternalLazyIMT.sol';

contract MyProgram is IE3Program, Ownable {
  using InternalLazyIMT for LazyIMTData;
  // Constants
  bytes32 public constant ENCRYPTION_SCHEME_ID = keccak256('fhe.rs:BFV');

  uint8 public constant TREE_DEPTH = 20;

  // State variables
  IEnclave public enclave;
  IRiscZeroVerifier public verifier;
  bytes32 public imageId;

  // Mappings
  mapping(address => bool) public authorizedContracts;
  mapping(uint256 e3Id => bytes32 paramsHash) public paramsHashes;
  mapping(uint256 e3Id => LazyIMTData) public inputs;

  // Errors
  error CallerNotAuthorized();
  error E3AlreadyInitialized();
  error E3DoesNotExist();
  error VerifierAddressZero();
  error AlreadyRegistered();
  error EmptyInputData();

  event InputPublished(uint256 indexed e3Id, bytes data, uint256 index);

  /// @notice Initialize the contract, binding it to a specified RISC Zero verifier.
  /// @param _enclave The Enclave contract address
  /// @param _verifier The RISC Zero verifier address
  /// @param _imageId The image ID for the guest program
  constructor(IEnclave _enclave, IRiscZeroVerifier _verifier, bytes32 _imageId) Ownable(msg.sender) {
    require(address(_verifier) != address(0), VerifierAddressZero());

    enclave = _enclave;
    verifier = _verifier;
    imageId = _imageId;
    authorizedContracts[address(_enclave)] = true;
  }

  /// @notice Validate the E3 program parameters
  /// @param e3Id The E3 program ID
  /// @param e3ProgramParams The E3 program parameters
  function validate(uint256 e3Id, uint256, bytes calldata e3ProgramParams, bytes calldata) external returns (bytes32) {
    require(authorizedContracts[msg.sender] || msg.sender == owner(), CallerNotAuthorized());
    require(paramsHashes[e3Id] == bytes32(0), E3AlreadyInitialized());
    paramsHashes[e3Id] = keccak256(e3ProgramParams);

    inputs[e3Id]._init(TREE_DEPTH);

    return ENCRYPTION_SCHEME_ID;
  }

  /// @notice Validates input
  /// @param sender The account that is submitting the input.
  /// @param data The input to be verified.
  /// @return input The input data.
  function validateInput(uint256 e3Id, address sender, bytes memory data) external returns (bytes memory input) {
    if (data.length == 0) revert EmptyInputData();

    // You can add your own validation logic here.
    // EXAMPLE: https://github.com/gnosisguild/enclave/blob/main/examples/CRISP/packages/crisp-contracts/contracts/CRISPProgram.sol

    input = data;

    uint256 index = inputs[e3Id].numberOfLeaves;
    inputs[e3Id]._insert(PoseidonT3.hash([uint256(keccak256(data)), index]));

    emit InputPublished(e3Id, data, index);
  }

  /// @notice Verify the proof
  /// @param e3Id The E3 program ID
  /// @param ciphertextOutputHash The hash of the ciphertext output
  /// @param proof The proof to verify
  function verify(uint256 e3Id, bytes32 ciphertextOutputHash, bytes memory proof) external view override returns (bool) {
    require(paramsHashes[e3Id] != bytes32(0), E3DoesNotExist());
    bytes32 inputRoot = bytes32(inputs[e3Id]._root());
    bytes memory journal = new bytes(396); // (32 + 1) * 4 * 3

    encodeLengthPrefixAndHash(journal, 0, ciphertextOutputHash);
    encodeLengthPrefixAndHash(journal, 132, paramsHashes[e3Id]);
    encodeLengthPrefixAndHash(journal, 264, inputRoot);

    verifier.verify(proof, imageId, sha256(journal));
    return true;
  }

  /// @notice Encode length prefix and hash
  /// @param journal The journal to encode into
  /// @param startIndex The start index in the journal
  /// @param hashVal The hash value to encode
  function encodeLengthPrefixAndHash(bytes memory journal, uint256 startIndex, bytes32 hashVal) internal pure {
    journal[startIndex] = 0x20;
    startIndex += 4;
    for (uint256 i = 0; i < 32; i++) {
      journal[startIndex + i * 4] = hashVal[i];
    }
  }
}
