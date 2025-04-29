// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {ImageID} from "./ImageID.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {IE3Program} from "@gnosis-guild/enclave/contracts/interfaces/IE3Program.sol";
import {IEnclavePolicy} from "@gnosis-guild/enclave/contracts/interfaces/IEnclavePolicy.sol";
import {IEnclave} from "@gnosis-guild/enclave/contracts/interfaces/IEnclave.sol";
import {ISemaphore} from "@semaphore-protocol/contracts/interfaces/ISemaphore.sol";
import {CRISPCheckerFactory} from "./CRISPCheckerFactory.sol";
import {CRISPPolicyFactory} from "./CRISPPolicyFactory.sol";

contract CRISPRisc0 is IE3Program, Ownable {
    // Constants
    bytes32 public constant IMAGE_ID = ImageID.VOTING_ID;
    bytes32 public constant ENCRYPTION_SCHEME_ID = keccak256("fhe.rs:BFV");

    // State variables
    IEnclave public enclave;
    IRiscZeroVerifier public verifier;
    ISemaphore public semaphore;
    CRISPCheckerFactory private immutable CHECKER_FACTORY;
    CRISPPolicyFactory private immutable POLICY_FACTORY;
    uint8 public constant INPUT_LIMIT = 100;

    // Mappings
    mapping(address => bool) public authorizedContracts;
    mapping(uint256 e3Id => bytes32 paramsHash) public paramsHashes;
    mapping(uint256 e3Id => uint256 groupId) public groupIds;
    // Could Save These in the Policy Contract and add a Pre-Check
    mapping(uint256 groupId => mapping(uint256 identityCommitment => bool))
        public committed;
    mapping(uint256 groupId => uint256[]) public groupCommitments;

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
    error GroupDoesNotExist();
    error AlreadyRegistered();

    /// @notice Initialize the contract, binding it to a specified RISC Zero verifier.
    /// @param _enclave The enclave address
    /// @param _verifier The RISC Zero verifier address
    /// @param _semaphore The Semaphore address
    /// @param _checkerFactory The checker factory address
    /// @param _policyFactory The policy factory address
    constructor(
        IEnclave _enclave,
        IRiscZeroVerifier _verifier,
        ISemaphore _semaphore,
        CRISPCheckerFactory _checkerFactory,
        CRISPPolicyFactory _policyFactory
    ) Ownable(msg.sender) {
        require(address(_enclave) != address(0), EnclaveAddressZero());
        require(address(_verifier) != address(0), VerifierAddressZero());
        require(address(_semaphore) != address(0), SemaphoreAddressZero());
        require(
            address(_checkerFactory) != address(0),
            InvalidCheckerFactory()
        );
        require(address(_policyFactory) != address(0), InvalidPolicyFactory());

        enclave = _enclave;
        verifier = _verifier;
        semaphore = _semaphore;
        CHECKER_FACTORY = _checkerFactory;
        POLICY_FACTORY = _policyFactory;
        authorizedContracts[address(_enclave)] = true;
    }

    /// @notice Register a Member to the semaphore group
    /// @param e3Id The E3 program ID
    /// @param identityCommitment The identity commitment
    function registerMember(uint256 e3Id, uint256 identityCommitment) external {
        require(paramsHashes[e3Id] != bytes32(0), GroupDoesNotExist());
        uint256 groupId = groupIds[e3Id];

        require(!committed[groupId][identityCommitment], AlreadyRegistered());
        committed[groupId][identityCommitment] = true;
        groupCommitments[groupId].push(identityCommitment);

        semaphore.addMember(groupId, identityCommitment);
    }

    /// @notice Get the group commitments
    /// @param groupId The group ID
    /// @return The group commitments
    function getGroupCommitments(
        uint256 groupId
    ) public view returns (uint256[] memory) {
        return groupCommitments[groupId];
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
    ) external returns (bytes32, IEnclavePolicy inputValidator) {
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
        inputValidator = IEnclavePolicy(
            POLICY_FACTORY.deploy(checker, INPUT_LIMIT)
        );
        inputValidator.setTarget(msg.sender);

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

        verifier.verify(proof, IMAGE_ID, sha256(journal));
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
