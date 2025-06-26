// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {IE3Program} from "@gnosis-guild/enclave/contracts/interfaces/IE3Program.sol";
import {IBasePolicy} from "@excubiae/contracts/interfaces/IBasePolicy.sol";
import {IInputValidator} from "@gnosis-guild/enclave/contracts/interfaces/IInputValidator.sol";
import {IEnclave} from "@gnosis-guild/enclave/contracts/interfaces/IEnclave.sol";
import {ISemaphore} from "@semaphore-protocol/contracts-noir/interfaces/ISemaphoreNoir.sol";
import {CRISPCheckerNoirFactory} from "./CRISPCheckerNoirFactory.sol";
import {CRISPPolicyNoirFactory} from "./CRISPPolicyNoirFactory.sol";
import {CRISPInputValidatorFactory} from "./CRISPInputValidatorFactory.sol";
import {HonkVerifier} from "./CRISPVerifier.sol";

contract CRISPProgram is IE3Program, Ownable {
    // Constants
    bytes32 public constant ENCRYPTION_SCHEME_ID = keccak256("fhe.rs:BFV");

    // State variables
    IEnclave public enclave;
    IRiscZeroVerifier public verifier;
    ISemaphore public semaphore;
    CRISPCheckerNoirFactory private immutable CHECKER_FACTORY;
    CRISPPolicyNoirFactory private immutable POLICY_FACTORY;
    CRISPInputValidatorFactory private immutable INPUT_VALIDATOR_FACTORY;
    HonkVerifier private immutable HONK_VERIFIER;
    uint8 public constant INPUT_LIMIT = 100;
    bytes32 public imageId;

    // Mappings
    mapping(address => bool) public authorizedContracts;
    mapping(uint256 e3Id => bytes32 paramsHash) public paramsHashes;
    mapping(uint256 e3Id => uint256 groupId) public groupIds;
    mapping(uint256 groupId => mapping(uint256 identityCommitment => bool))
        public committed;

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
    error InvalidHonkVerifier();
    error GroupDoesNotExist();
    error AlreadyRegistered();

    /// @notice Initialize the contract, binding it to a specified RISC Zero verifier.
    /// @param _enclave The enclave address
    /// @param _verifier The RISC Zero verifier address
    /// @param _semaphore The Semaphore address
    /// @param _checkerFactory The checker factory address
    /// @param _policyFactory The policy factory address
    /// @param _inputValidatorFactory The input validator factory address
    /// @param _honkVerifier The honk verifier address
    /// @param _imageId The image ID for the guest program
    constructor(
        IEnclave _enclave,
        IRiscZeroVerifier _verifier,
        ISemaphore _semaphore,
        CRISPCheckerNoirFactory _checkerFactory,
        CRISPPolicyNoirFactory _policyFactory,
        CRISPInputValidatorFactory _inputValidatorFactory,
        HonkVerifier _honkVerifier,
        bytes32 _imageId
    ) Ownable(msg.sender) {
        require(address(_enclave) != address(0), EnclaveAddressZero());
        require(address(_verifier) != address(0), VerifierAddressZero());
        require(address(_semaphore) != address(0), SemaphoreAddressZero());
        require(
            address(_checkerFactory) != address(0),
            InvalidCheckerFactory()
        );
        require(address(_policyFactory) != address(0), InvalidPolicyFactory());
        require(
            address(_inputValidatorFactory) != address(0),
            InvalidInputValidatorFactory()
        );
        require(address(_honkVerifier) != address(0), InvalidHonkVerifier());

        enclave = _enclave;
        verifier = _verifier;
        semaphore = _semaphore;
        CHECKER_FACTORY = _checkerFactory;
        POLICY_FACTORY = _policyFactory;
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

    /// @notice Register a Member to the semaphore group
    /// @param e3Id The E3 program ID
    /// @param identityCommitment The identity commitment
    function registerMember(uint256 e3Id, uint256 identityCommitment) external {
        require(paramsHashes[e3Id] != bytes32(0), GroupDoesNotExist());
        uint256 groupId = groupIds[e3Id];

        require(!committed[groupId][identityCommitment], AlreadyRegistered());
        committed[groupId][identityCommitment] = true;

        semaphore.addMember(groupId, identityCommitment);
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

        // Create a new group
        uint256 groupId = semaphore.createGroup(address(this));
        groupIds[e3Id] = groupId;

        // Deploy a new checker
        address checker = CHECKER_FACTORY.deploy(address(semaphore), groupId);

        // Deploy a new policy
        IBasePolicy policy = IBasePolicy(
            POLICY_FACTORY.deploy(msg.sender, checker, INPUT_LIMIT)
        );

        // Deploy a new input validator
        inputValidator = IInputValidator(
            INPUT_VALIDATOR_FACTORY.deploy(
                address(policy),
                address(HONK_VERIFIER)
            )
        );
        policy.setTarget(address(inputValidator));

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
