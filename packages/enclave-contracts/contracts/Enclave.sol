// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.
pragma solidity >=0.8.27;

import { IEnclave, E3, IE3Program } from "./interfaces/IEnclave.sol";
import { IInputValidator } from "./interfaces/IInputValidator.sol";
import { ICiphernodeRegistry } from "./interfaces/ICiphernodeRegistry.sol";
import { IDecryptionVerifier } from "./interfaces/IDecryptionVerifier.sol";
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";
import {
    InternalLeanIMT,
    LeanIMTData,
    PoseidonT3
} from "@zk-kit/lean-imt.sol/InternalLeanIMT.sol";

contract Enclave is IEnclave, OwnableUpgradeable {
    using InternalLeanIMT for LeanIMTData;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                 Storage Variables                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    ICiphernodeRegistry public ciphernodeRegistry; // address of the Ciphernode registry.
    uint256 public maxDuration; // maximum duration of a computation in seconds.
    uint256 public nexte3Id; // ID of the next E3.
    uint256 public requests; // total number of requests made to Enclave.

    // Mapping of allowed E3 Programs.
    mapping(IE3Program e3Program => bool allowed) public e3Programs;

    // Mapping of E3s.
    mapping(uint256 e3Id => E3 e3) public e3s;

    // Mapping of input merkle trees.
    mapping(uint256 e3Id => LeanIMTData imt) public inputs;

    // Mapping counting the number of inputs for each E3.
    mapping(uint256 e3Id => uint256 inputCount) public inputCounts;

    // Mapping of enabled encryption schemes.
    mapping(bytes32 encryptionSchemeId => IDecryptionVerifier decryptionVerifier)
        public decryptionVerifiers;

    /// Mapping that stores the valid E3 program ABI encoded parameter sets (e.g., BFV).
    mapping(bytes e3ProgramParams => bool allowed) public e3ProgramsParams;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Errors                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    error CommitteeSelectionFailed();
    error E3ProgramNotAllowed(IE3Program e3Program);
    error E3AlreadyActivated(uint256 e3Id);
    error E3Expired();
    error E3NotActivated(uint256 e3Id);
    error E3NotReady();
    error E3DoesNotExist(uint256 e3Id);
    error ModuleAlreadyEnabled(address module);
    error ModuleNotEnabled(address module);
    error InvalidEncryptionScheme(bytes32 encryptionSchemeId);
    error InputDeadlinePassed(uint256 e3Id, uint256 expiration);
    error InputDeadlineNotPassed(uint256 e3Id, uint256 expiration);
    error InvalidComputationRequest(IInputValidator inputValidator);
    error InvalidCiphernodeRegistry(ICiphernodeRegistry ciphernodeRegistry);
    error InvalidDuration(uint256 duration);
    error InvalidOutput(bytes output);
    error InvalidInput();
    error InvalidStartWindow();
    error InvalidThreshold(uint32[2] threshold);
    error CiphertextOutputAlreadyPublished(uint256 e3Id);
    error CiphertextOutputNotPublished(uint256 e3Id);
    error PaymentRequired(uint256 value);
    error PlaintextOutputAlreadyPublished(uint256 e3Id);

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Initialization                       //
    //                                                        //
    ////////////////////////////////////////////////////////////

    /// @param _owner The owner of this contract
    /// @param _maxDuration The maximum duration of a computation in seconds
    /// @param _e3ProgramsParams Array of ABI encoded E3 encryption scheme parameters sets (e.g., for BFV)
    constructor(
        address _owner,
        ICiphernodeRegistry _ciphernodeRegistry,
        uint256 _maxDuration,
        bytes[] memory _e3ProgramsParams
    ) {
        initialize(
            _owner,
            _ciphernodeRegistry,
            _maxDuration,
            _e3ProgramsParams
        );
    }

    /// @param _owner The owner of this contract
    /// @param _ciphernodeRegistry The address of the ciphernode registry
    /// @param _maxDuration The maximum duration of a computation in seconds
    /// @param _e3ProgramsParams Array of ABI encoded E3 encryption scheme parameters sets (e.g., for BFV)
    function initialize(
        address _owner,
        ICiphernodeRegistry _ciphernodeRegistry,
        uint256 _maxDuration,
        bytes[] memory _e3ProgramsParams
    ) public initializer {
        __Ownable_init(msg.sender);
        setMaxDuration(_maxDuration);
        setCiphernodeRegistry(_ciphernodeRegistry);
        setE3ProgramsParams(_e3ProgramsParams);
        if (_owner != owner()) transferOwnership(_owner);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                  Core Entrypoints                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    function request(
        E3RequestParams calldata requestParams
    ) external payable returns (uint256 e3Id, E3 memory e3) {
        // TODO: allow for other payment methods or only native tokens?
        // TODO: should payment checks be somewhere else? Perhaps in the E3 Program or ciphernode registry?
        require(msg.value > 0, PaymentRequired(msg.value));
        require(
            requestParams.threshold[1] >= requestParams.threshold[0] &&
                requestParams.threshold[0] > 0,
            InvalidThreshold(requestParams.threshold)
        );
        require(
            // TODO: do we need a minimum start window to allow time for committee selection?
            requestParams.startWindow[1] >= requestParams.startWindow[0] &&
                requestParams.startWindow[1] >= block.timestamp,
            InvalidStartWindow()
        );
        require(
            requestParams.duration > 0 && requestParams.duration <= maxDuration,
            InvalidDuration(requestParams.duration)
        );
        require(
            e3Programs[requestParams.e3Program],
            E3ProgramNotAllowed(requestParams.e3Program)
        );

        // TODO: should IDs be incremental or produced deterministically?
        e3Id = nexte3Id;
        nexte3Id++;
        uint256 seed = uint256(keccak256(abi.encode(block.prevrandao, e3Id)));

        (
            bytes32 encryptionSchemeId,
            IInputValidator inputValidator
        ) = requestParams.e3Program.validate(
                e3Id,
                seed,
                requestParams.e3ProgramParams,
                requestParams.computeProviderParams
            );
        IDecryptionVerifier decryptionVerifier = decryptionVerifiers[
            encryptionSchemeId
        ];

        require(
            decryptionVerifiers[encryptionSchemeId] !=
                IDecryptionVerifier(address(0)),
            InvalidEncryptionScheme(encryptionSchemeId)
        );
        require(
            address(inputValidator) != address(0),
            InvalidComputationRequest(inputValidator)
        );

        e3 = E3({
            seed: seed,
            threshold: requestParams.threshold,
            requestBlock: block.number,
            startWindow: requestParams.startWindow,
            duration: requestParams.duration,
            expiration: 0,
            encryptionSchemeId: encryptionSchemeId,
            e3Program: requestParams.e3Program,
            e3ProgramParams: requestParams.e3ProgramParams,
            inputValidator: inputValidator,
            decryptionVerifier: decryptionVerifier,
            committeePublicKey: hex"",
            ciphertextOutput: hex"",
            plaintextOutput: hex""
        });
        e3s[e3Id] = e3;

        require(
            ciphernodeRegistry.requestCommittee(
                e3Id,
                requestParams.filter,
                requestParams.threshold
            ),
            CommitteeSelectionFailed()
        );

        emit E3Requested(
            e3Id,
            e3,
            requestParams.filter,
            requestParams.e3Program
        );
    }

    function activate(
        uint256 e3Id,
        bytes calldata publicKey
    ) external returns (bool success) {
        E3 memory e3 = getE3(e3Id);

        require(e3.expiration == 0, E3AlreadyActivated(e3Id));
        require(e3.startWindow[0] <= block.timestamp, E3NotReady());
        // TODO: handle what happens to the payment if the start window has passed.
        require(e3.startWindow[1] >= block.timestamp, E3Expired());

        bytes32 publicKeyHash = ciphernodeRegistry.committeePublicKey(e3Id);
        require(
            keccak256(publicKey) == publicKeyHash,
            CommitteeSelectionFailed()
        );
        uint256 expiresAt = block.timestamp + e3.duration;
        e3s[e3Id].expiration = expiresAt;
        e3s[e3Id].committeePublicKey = keccak256(publicKey);

        emit E3Activated(e3Id, expiresAt, publicKey);

        return true;
    }

    function publishInput(
        uint256 e3Id,
        bytes calldata data
    ) external returns (bool success) {
        E3 memory e3 = getE3(e3Id);

        // Note: if we make 0 a no expiration, this has to be refactored
        require(e3.expiration > 0, E3NotActivated(e3Id));
        // TODO: should we have an input window, including both a start and end timestamp?
        require(
            e3.expiration > block.timestamp,
            InputDeadlinePassed(e3Id, e3.expiration)
        );

        bytes memory input = e3.inputValidator.validate(msg.sender, data);
        uint256 inputHash = PoseidonT3.hash(
            [uint256(keccak256(input)), inputCounts[e3Id]]
        );

        inputCounts[e3Id]++;
        inputs[e3Id]._insert(inputHash);
        success = true;

        emit InputPublished(e3Id, input, inputHash, inputCounts[e3Id] - 1);
    }

    function publishCiphertextOutput(
        uint256 e3Id,
        bytes calldata ciphertextOutput,
        bytes calldata proof
    ) external returns (bool success) {
        E3 memory e3 = getE3(e3Id);
        // Note: if we make 0 a no expiration, this has to be refactored
        require(e3.expiration > 0, E3NotActivated(e3Id));
        require(
            e3.expiration <= block.timestamp,
            InputDeadlineNotPassed(e3Id, e3.expiration)
        );
        // TODO: should the output verifier be able to change its mind?
        //i.e. should we be able to call this multiple times?
        require(
            e3.ciphertextOutput == bytes32(0),
            CiphertextOutputAlreadyPublished(e3Id)
        );
        bytes32 ciphertextOutputHash = keccak256(ciphertextOutput);
        (success) = e3.e3Program.verify(e3Id, ciphertextOutputHash, proof);
        require(success, InvalidOutput(ciphertextOutput));
        e3s[e3Id].ciphertextOutput = ciphertextOutputHash;

        emit CiphertextOutputPublished(e3Id, ciphertextOutput);
    }

    function publishPlaintextOutput(
        uint256 e3Id,
        bytes calldata plaintextOutput,
        bytes calldata proof
    ) external returns (bool success) {
        E3 memory e3 = getE3(e3Id);
        // Note: if we make 0 a no expiration, this has to be refactored
        require(e3.expiration > 0, E3NotActivated(e3Id));
        require(
            e3.ciphertextOutput != bytes32(0),
            CiphertextOutputNotPublished(e3Id)
        );
        require(
            e3.plaintextOutput.length == 0,
            PlaintextOutputAlreadyPublished(e3Id)
        );
        (success) = e3.decryptionVerifier.verify(
            e3Id,
            keccak256(plaintextOutput),
            proof
        );
        require(success, InvalidOutput(plaintextOutput));
        e3s[e3Id].plaintextOutput = plaintextOutput;

        emit PlaintextOutputPublished(e3Id, plaintextOutput);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Set Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    function setMaxDuration(
        uint256 _maxDuration
    ) public onlyOwner returns (bool success) {
        maxDuration = _maxDuration;
        success = true;
        emit MaxDurationSet(_maxDuration);
    }

    function setCiphernodeRegistry(
        ICiphernodeRegistry _ciphernodeRegistry
    ) public onlyOwner returns (bool success) {
        require(
            address(_ciphernodeRegistry) != address(0) &&
                _ciphernodeRegistry != ciphernodeRegistry,
            InvalidCiphernodeRegistry(_ciphernodeRegistry)
        );
        ciphernodeRegistry = _ciphernodeRegistry;
        success = true;
        emit CiphernodeRegistrySet(address(_ciphernodeRegistry));
    }

    function enableE3Program(
        IE3Program e3Program
    ) public onlyOwner returns (bool success) {
        require(
            !e3Programs[e3Program],
            ModuleAlreadyEnabled(address(e3Program))
        );
        e3Programs[e3Program] = true;
        success = true;
        emit E3ProgramEnabled(e3Program);
    }

    function disableE3Program(
        IE3Program e3Program
    ) public onlyOwner returns (bool success) {
        require(e3Programs[e3Program], ModuleNotEnabled(address(e3Program)));
        delete e3Programs[e3Program];
        success = true;
        emit E3ProgramDisabled(e3Program);
    }

    function setDecryptionVerifier(
        bytes32 encryptionSchemeId,
        IDecryptionVerifier decryptionVerifier
    ) public onlyOwner returns (bool success) {
        require(
            decryptionVerifier != IDecryptionVerifier(address(0)) &&
                decryptionVerifiers[encryptionSchemeId] != decryptionVerifier,
            InvalidEncryptionScheme(encryptionSchemeId)
        );
        decryptionVerifiers[encryptionSchemeId] = decryptionVerifier;
        success = true;
        emit EncryptionSchemeEnabled(encryptionSchemeId);
    }

    function disableEncryptionScheme(
        bytes32 encryptionSchemeId
    ) public onlyOwner returns (bool success) {
        require(
            decryptionVerifiers[encryptionSchemeId] !=
                IDecryptionVerifier(address(0)),
            InvalidEncryptionScheme(encryptionSchemeId)
        );
        decryptionVerifiers[encryptionSchemeId] = IDecryptionVerifier(
            address(0)
        );
        success = true;
        emit EncryptionSchemeDisabled(encryptionSchemeId);
    }

    function setE3ProgramsParams(
        bytes[] memory _e3ProgramsParams
    ) public onlyOwner returns (bool success) {
        uint256 length = _e3ProgramsParams.length;
        for (uint256 i; i < length; ) {
            e3ProgramsParams[_e3ProgramsParams[i]] = true;
            unchecked {
                ++i;
            }
        }
        success = true;
        emit AllowedE3ProgramsParamsSet(_e3ProgramsParams);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Get Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    function getE3(uint256 e3Id) public view returns (E3 memory e3) {
        e3 = e3s[e3Id];
        require(e3.e3Program != IE3Program(address(0)), E3DoesNotExist(e3Id));
    }

    function getInputRoot(uint256 e3Id) public view returns (uint256) {
        require(
            e3s[e3Id].e3Program != IE3Program(address(0)),
            E3DoesNotExist(e3Id)
        );
        return InternalLeanIMT._root(inputs[e3Id]);
    }

    function getDecryptionVerifier(
        bytes32 encryptionSchemeId
    ) public view returns (IDecryptionVerifier) {
        return decryptionVerifiers[encryptionSchemeId];
    }
}
