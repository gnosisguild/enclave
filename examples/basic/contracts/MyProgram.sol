// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {IRiscZeroVerifier} from "@risc0/ethereum/contracts/src/IRiscZeroVerifier.sol";
import {IE3Program} from "@gnosis-guild/enclave/contracts/interfaces/IE3Program.sol";
import {IInputValidator} from "@gnosis-guild/enclave/contracts/interfaces/IInputValidator.sol";
import {IEnclave} from "@gnosis-guild/enclave/contracts/interfaces/IEnclave.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";

contract MyProgram is IE3Program, Ownable {
    // Constants
    bytes32 public constant ENCRYPTION_SCHEME_ID = keccak256("fhe.rs:BFV");

    // State variables
    IEnclave public enclave;
    IRiscZeroVerifier public verifier;
    bytes32 public imageId;

    // Mappings
    mapping(address => bool) public authorizedContracts;
    mapping(uint256 e3Id => bytes32 paramsHash) public paramsHashes;

    // Events
    event InputValidatorUpdated(address indexed newValidator);

    // Errors
    error CallerNotAuthorized();
    error E3AlreadyInitialized();
    error E3DoesNotExist();
    error EnclaveAddressZero();
    error VerifierAddressZero();
    error SemaphoreAddressZero();
    error InvalidPolicyFactory();
    error InvalidCheckerFactory();
    error InvalidInputValidatorFactory();
    error GroupDoesNotExist();
    error AlreadyRegistered();

    /// @notice Initialize the contract, binding it to a specified RISC Zero verifier.
    /// @param _enclave The enclave address
    /// @param _verifier The RISC Zero verifier address
    /// @param _imageId The image ID for the guest program
    constructor(
        IEnclave _enclave,
        IRiscZeroVerifier _verifier,
        bytes32 _imageId
    ) Ownable(msg.sender) {
        require(address(_enclave) != address(0), EnclaveAddressZero());
        require(address(_verifier) != address(0), VerifierAddressZero());

        enclave = _enclave;
        verifier = _verifier;
        imageId = _imageId;
    }

    /// @notice Validate the E3 program parameters
    /// @param e3Id The E3 program ID
    /// @param e3ProgramParams The E3 program parameters
    function validate(
        uint256 e3Id,
        uint256,
        bytes calldata e3ProgramParams,
        bytes calldata
    ) external returns (bytes32, IInputValidator inputValidator) {
        require(
            authorizedContracts[msg.sender] || msg.sender == owner(),
            CallerNotAuthorized()
        );
        require(paramsHashes[e3Id] == bytes32(0), E3AlreadyInitialized());
        paramsHashes[e3Id] = keccak256(e3ProgramParams);

        return (ENCRYPTION_SCHEME_ID, inputValidator);
    }

    /// @notice Verify the proof
    /// @param e3Id The E3 program ID
    /// @param ciphertextOutputHash The hash of the ciphertext output
    /// @param proof The proof to verify
    function verify(
        uint256 e3Id,
        bytes32 ciphertextOutputHash,
        bytes memory proof
    ) external view override returns (bool) {
        require(paramsHashes[e3Id] != bytes32(0), E3DoesNotExist());
        bytes32 inputRoot = bytes32(enclave.getInputRoot(e3Id));
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
    function encodeLengthPrefixAndHash(
        bytes memory journal,
        uint256 startIndex,
        bytes32 hashVal
    ) internal pure {
        journal[startIndex] = 0x20;
        startIndex += 4;
        for (uint256 i = 0; i < 32; i++) {
            journal[startIndex + i * 4] = hashVal[i];
        }
    }
}
