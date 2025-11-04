// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {IE3Program} from "@enclave-e3/contracts/contracts/interfaces/IE3Program.sol";
import {IInputValidator} from "@enclave-e3/contracts/contracts/interfaces/IInputValidator.sol";
import {IEnclave} from "@enclave-e3/contracts/contracts/interfaces/IEnclave.sol";
import {E3} from "@enclave-e3/contracts/contracts/interfaces/IE3.sol";
import {CRISPInputValidatorFactory} from "./CRISPInputValidatorFactory.sol";
import {HonkVerifier} from "./CRISPVerifier.sol";

contract CRISPProgram is IE3Program, Ownable {
    // Constants
    bytes32 public constant ENCRYPTION_SCHEME_ID = keccak256("fhe.rs:BFV");

    // State variables
    IEnclave public enclave;
    IRiscZeroVerifier public verifier;
    CRISPInputValidatorFactory private immutable INPUT_VALIDATOR_FACTORY;
    HonkVerifier private immutable HONK_VERIFIER;
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
    error InvalidInputValidatorFactory();
    error InvalidHonkVerifier();

    /// @notice Initialize the contract, binding it to a specified RISC Zero verifier.
    /// @param _enclave The enclave address
    /// @param _verifier The RISC Zero verifier address
    /// @param _inputValidatorFactory The input validator factory address
    /// @param _honkVerifier The honk verifier address
    /// @param _imageId The image ID for the guest program
    constructor(
        IEnclave _enclave,
        IRiscZeroVerifier _verifier,
        CRISPInputValidatorFactory _inputValidatorFactory,
        HonkVerifier _honkVerifier,
        bytes32 _imageId
    ) Ownable(msg.sender) {
        require(address(_enclave) != address(0), EnclaveAddressZero());
        require(address(_verifier) != address(0), VerifierAddressZero());
        require(
            address(_inputValidatorFactory) != address(0),
            InvalidInputValidatorFactory()
        );
        require(address(_honkVerifier) != address(0), InvalidHonkVerifier());

        enclave = _enclave;
        verifier = _verifier;
        INPUT_VALIDATOR_FACTORY = _inputValidatorFactory;
        HONK_VERIFIER = _honkVerifier;
        authorizedContracts[address(_enclave)] = true;
        imageId = _imageId;
    }

    /// @notice Set the Image ID for the guest program
    /// @param _imageId The new image ID.
    function setImageId(bytes32 _imageId) external onlyOwner {
        imageId = _imageId;
    }

    /// @notice Set the RISC Zero verifier address
    /// @param _verifier The new RISC Zero verifier address
    function setVerifier(IRiscZeroVerifier _verifier) external onlyOwner {
        verifier = _verifier;
    }

    /// @notice Get the params hash for an E3 program
    /// @param e3Id The E3 program ID
    /// @return The params hash
    function getParamsHash(uint256 e3Id) public view returns (bytes32) {
        return paramsHashes[e3Id];
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

        // Deploy a new input validator
        inputValidator = IInputValidator(
            INPUT_VALIDATOR_FACTORY.deploy(address(HONK_VERIFIER), owner())
        );

        return (ENCRYPTION_SCHEME_ID, inputValidator);
    }

    /// @notice Decode the tally from the plaintext output
    function decodeTally(uint256 e3Id) public returns (uint256 yes, uint256 no) {
        // fetch from enclave 
        E3 memory e3 = enclave.getE3(e3Id);

        uint256[] memory tally = abi.decode(e3.plaintextOutput, (uint256[]));

        uint256 HALF_LARGEST_MINIMUM_DEGREE = 28;
        uint256 HALF_D = tally.length / 2;
        uint256 START_INDEX_Y = HALF_D - HALF_LARGEST_MINIMUM_DEGREE;
        uint256 START_INDEX_N = tally.length - HALF_LARGEST_MINIMUM_DEGREE;
    
        uint256 weight = 2 ** (HALF_LARGEST_MINIMUM_DEGREE - 1);
        
        // Convert yes votes
        for (uint256 i = START_INDEX_Y; i < HALF_D; i++) {
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
