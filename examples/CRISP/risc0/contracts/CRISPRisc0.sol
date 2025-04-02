// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {IRiscZeroVerifier} from "risc0/IRiscZeroVerifier.sol";
import {ImageID} from "./ImageID.sol";
import {Ownable} from "@openzeppelin/contracts/access/Ownable.sol";
import {IE3Program, IEnclavePolicy} from "@gnosis-guild/enclave/contracts/interfaces/IE3Program.sol";
import {IEnclave} from "@gnosis-guild/enclave/contracts/interfaces/IEnclave.sol";

contract CRISPRisc0 is IE3Program, Ownable {
    // Constants
    bytes32 public constant IMAGE_ID = ImageID.VOTING_ID;
    bytes32 public constant ENCRYPTION_SCHEME_ID = keccak256("fhe.rs:BFV");

    // State variables
    IEnclave public enclave;
    IRiscZeroVerifier public verifier;
    IEnclavePolicy public policy;

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

    /// @notice Initialize the contract, binding it to a specified RISC Zero verifier.
    /// @param _enclave The enclave address
    /// @param _policy The enclave policy address
    /// @param _verifier The RISC Zero verifier address
    constructor(
        IEnclave _enclave,
        IEnclavePolicy _policy,
        IRiscZeroVerifier _verifier
    ) Ownable(msg.sender) {
        initialize(_enclave, _policy, _verifier);
    }

    /// @notice Initialize the contract components
    /// @param _enclave The enclave address
    /// @param _policy The enclave policy address
    /// @param _verifier The RISC Zero verifier address
    function initialize(
        IEnclave _enclave,
        IEnclavePolicy _policy,
        IRiscZeroVerifier _verifier
    ) public {
        require(address(enclave) == address(0), EnclaveAddressZero());
        enclave = _enclave;
        policy = _policy;
        verifier = _verifier;
        authorizedContracts[address(_enclave)] = true;
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
        uint8 inputLimit,
        bytes calldata e3ProgramParams,
        bytes calldata
    ) external returns (bytes32, IEnclavePolicy) {
        require(
            authorizedContracts[msg.sender] || msg.sender == owner(),
            CallerNotAuthorized()
        );
        require(paramsHashes[e3Id] == bytes32(0), E3AlreadyInitialized());

        paramsHashes[e3Id] = keccak256(e3ProgramParams);

        return (ENCRYPTION_SCHEME_ID, policy);
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

