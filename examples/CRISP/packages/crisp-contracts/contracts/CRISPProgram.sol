// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IRiscZeroVerifier } from "risc0/IRiscZeroVerifier.sol";
import { Ownable } from "@openzeppelin/contracts/access/Ownable.sol";
import { IE3Program } from "@enclave-e3/contracts/contracts/interfaces/IE3Program.sol";
import { IEnclave } from "@enclave-e3/contracts/contracts/interfaces/IEnclave.sol";
import { E3 } from "@enclave-e3/contracts/contracts/interfaces/IE3.sol";
import { LazyIMTData, InternalLazyIMT, PoseidonT3 } from "@zk-kit/lazy-imt.sol/InternalLazyIMT.sol";
import { HonkVerifier } from "./CRISPVerifier.sol";

contract CRISPProgram is IE3Program, Ownable {
  using InternalLazyIMT for LazyIMTData;

  /// @notice Struct to store all data related to a voting round
  struct RoundData {
    uint256 merkleRoot;
    bytes32 paramsHash;
    mapping(address slot => uint40 index) voteSlots;
    LazyIMTData votes;
  }

  // Constants
  /// @notice Encryption scheme ID used for the CRISP program.
  bytes32 public constant ENCRYPTION_SCHEME_ID = keccak256("fhe.rs:BFV");
  /// @notice The depth of the input Merkle tree.
  uint8 public constant TREE_DEPTH = 20;
  /// @notice Half of the largest minimum degree used to fit votes inside the plaintext polynomial.
  uint256 public constant HALF_LARGEST_MINIMUM_DEGREE = 28; // static, hardcoded in the circuit.

  // State variables
  IEnclave public enclave;
  IRiscZeroVerifier public risc0Verifier;
  bytes32 public imageId;
  HonkVerifier private immutable honkVerifier;

  // Mappings
  mapping(address => bool) public authorizedContracts;
  mapping(uint256 e3Id => RoundData) e3Data;

  // Errors
  error CallerNotAuthorized();
  error E3AlreadyInitialized();
  error E3DoesNotExist();
  error EnclaveAddressZero();
  error Risc0VerifierAddressZero();
  error InvalidHonkVerifier();
  error EmptyInputData();
  error InvalidNoirProof();
  error InvalidMerkleRoot();
  error MerkleRootAlreadySet();

  // Events
  event InputPublished(uint256 indexed e3Id, bytes vote, uint256 index);

  /// @notice Initialize the contract, binding it to a specified RISC Zero verifier.
  /// @param _enclave The enclave address
  /// @param _risc0Verifier The RISC Zero verifier address
  /// @param _honkVerifier The honk verifier address
  /// @param _imageId The image ID for the guest program
  constructor(IEnclave _enclave, IRiscZeroVerifier _risc0Verifier, HonkVerifier _honkVerifier, bytes32 _imageId) Ownable(msg.sender) {
    if (address(_enclave) == address(0)) revert EnclaveAddressZero();
    if (address(_risc0Verifier) == address(0)) revert Risc0VerifierAddressZero();
    if (address(_honkVerifier) == address(0)) revert InvalidHonkVerifier();

    enclave = _enclave;
    risc0Verifier = _risc0Verifier;
    honkVerifier = _honkVerifier;
    authorizedContracts[address(_enclave)] = true;
    imageId = _imageId;
  }

  /// @notice Sets the Merkle root for an E3 program. Can only be set once.
  /// @param _e3Id The E3 program ID
  /// @param _root The Merkle root to set.
  function setMerkleRoot(uint256 _e3Id, uint256 _root) external onlyOwner {
    if (_root == 0) revert InvalidMerkleRoot();
    if (e3Data[_e3Id].merkleRoot != 0) revert MerkleRootAlreadySet();

    e3Data[_e3Id].merkleRoot = _root;
  }

  /// @notice Set the Image ID for the guest program
  /// @param _imageId The new image ID.
  function setImageId(bytes32 _imageId) external onlyOwner {
    imageId = _imageId;
  }

  /// @notice Set the RISC Zero verifier.
  /// @param _risc0Verifier The new RISC Zero verifier address
  function setRisc0Verifier(IRiscZeroVerifier _risc0Verifier) external onlyOwner {
    if (address(_risc0Verifier) == address(0)) revert Risc0VerifierAddressZero();
    risc0Verifier = _risc0Verifier;
  }

  /// @notice Get the params hash for an E3 program
  /// @param e3Id The E3 program ID
  /// @return The params hash
  function getParamsHash(uint256 e3Id) public view returns (bytes32) {
    return e3Data[e3Id].paramsHash;
  }

  /// @inheritdoc IE3Program
  function validate(uint256 e3Id, uint256, bytes calldata e3ProgramParams, bytes calldata) external returns (bytes32) {
    if (!authorizedContracts[msg.sender] && msg.sender != owner()) revert CallerNotAuthorized();
    if (e3Data[e3Id].paramsHash != bytes32(0)) revert E3AlreadyInitialized();

    e3Data[e3Id].paramsHash = keccak256(e3ProgramParams);

    // Initialize the votes Merkle tree for this E3 ID.
    e3Data[e3Id].votes._init(TREE_DEPTH);

    return ENCRYPTION_SCHEME_ID;
  }

  /// @inheritdoc IE3Program
  function validateInput(uint256 e3Id, address, bytes memory data) external {
    // it should only be called via Enclave for now
    if (!authorizedContracts[msg.sender] && msg.sender != owner()) revert CallerNotAuthorized();

    // We need to ensure that the CRISP admin set the merkle root of the census.
    // TODO: Uncomment this when we make the merkle root a public input of the circuit.
    // if (e3Data[e3Id].merkleRoot == 0) revert MerkleRootNotSet();

    if (data.length == 0) revert EmptyInputData();

    (bytes memory noirProof, bytes32[] memory vote, address slotAddress) = abi.decode(data, (bytes, bytes32[], address));

    bytes memory voteBytes = abi.encode(vote);

    (uint40 voteIndex, bool isFirstVote) = _processVote(e3Id, slotAddress, voteBytes);

    // Set public inputs for the proof. Order must match Noir circuit.
    bytes32[] memory noirPublicInputs = new bytes32[](2 + vote.length);

    noirPublicInputs[0] = bytes32(uint256(uint160(slotAddress)));
    noirPublicInputs[1] = bytes32(uint256(isFirstVote ? 1 : 0));
    for (uint256 i = 0; i < vote.length; i++) {
      noirPublicInputs[i + 2] = vote[i];
    }

    // Check if the ciphertext was encrypted correctly
    if (!honkVerifier.verify(noirProof, noirPublicInputs)) {
      revert InvalidNoirProof();
    }

    emit InputPublished(e3Id, voteBytes, voteIndex);
  }

  /// @notice Decode the tally from the plaintext output
  /// @param e3Id The E3 program ID
  /// @return yes The number of yes votes
  /// @return no The number of no votes
  function decodeTally(uint256 e3Id) public view returns (uint256 yes, uint256 no) {
    // fetch from enclave
    E3 memory e3 = enclave.getE3(e3Id);

    // abi decode it into an array of uint256
    uint256[] memory tally = abi.decode(e3.plaintextOutput, (uint256[]));

    /// @notice We want to completely ignore anything outside of the coefficients
    /// we agreed to store out votes on.
    uint256 halfD = tally.length / 2;
    uint256 START_INDEX_Y = halfD - HALF_LARGEST_MINIMUM_DEGREE;
    uint256 START_INDEX_N = tally.length - HALF_LARGEST_MINIMUM_DEGREE;

    // first weight (we are converting back from bits to integer)
    uint256 weight = 2 ** (HALF_LARGEST_MINIMUM_DEGREE - 1);

    // Convert yes votes
    for (uint256 i = START_INDEX_Y; i < halfD; i++) {
      yes += tally[i] * weight;
      weight /= 2; // Right shift equivalent
    }

    // Reset weight for no votes
    weight = 2 ** (HALF_LARGEST_MINIMUM_DEGREE - 1);

    // Convert no votes
    for (uint256 i = START_INDEX_N; i < tally.length; i++) {
      no += tally[i] * weight;
      weight /= 2;
    }

    return (yes, no);
  }

  /// @inheritdoc IE3Program
  function verify(uint256 e3Id, bytes32 ciphertextOutputHash, bytes memory proof) external view override returns (bool) {
    if (e3Data[e3Id].paramsHash == bytes32(0)) revert E3DoesNotExist();
    bytes32 inputRoot = bytes32(e3Data[e3Id].votes._root());
    bytes memory journal = new bytes(396); // (32 + 1) * 4 * 3

    _encodeLengthPrefixAndHash(journal, 0, ciphertextOutputHash);
    _encodeLengthPrefixAndHash(journal, 132, e3Data[e3Id].paramsHash);
    _encodeLengthPrefixAndHash(journal, 264, inputRoot);

    risc0Verifier.verify(proof, imageId, sha256(journal));
    return true;
  }

  /// @notice Process a vote: insert or update in the merkle tree depending
  /// on whether it's the first vote or an override.
  function _processVote(uint256 e3Id, address slotAddress, bytes memory vote) internal returns (uint40 voteIndex, bool isFirstVote) {
    uint40 storedIndexPlusOne = e3Data[e3Id].voteSlots[slotAddress];

    // we treat the index 0 as not voted yet
    // any valid index will be index + 1
    if (storedIndexPlusOne == 0) {
      // FIRST VOTE
      isFirstVote = true;
      voteIndex = e3Data[e3Id].votes.numberOfLeaves;
      e3Data[e3Id].voteSlots[slotAddress] = voteIndex + 1;
      e3Data[e3Id].votes._insert(PoseidonT3.hash([uint256(keccak256(vote)), voteIndex]));
    } else {
      // RE-VOTE
      isFirstVote = false;
      voteIndex = storedIndexPlusOne - 1;
      e3Data[e3Id].votes._update(PoseidonT3.hash([uint256(keccak256(vote)), voteIndex]), voteIndex);
    }
  }

  /// @notice Encode length prefix and hash
  /// @param journal The journal to encode into
  /// @param startIndex The start index in the journal
  /// @param hashVal The hash value to encode
  function _encodeLengthPrefixAndHash(bytes memory journal, uint256 startIndex, bytes32 hashVal) internal pure {
    journal[startIndex] = 0x20;
    startIndex += 4;

    for (uint256 i = 0; i < 32; i++) {
      journal[startIndex + i * 4] = hashVal[i];
    }
  }
}
