// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.27;

import {
    IEnclave,
    E3,
    IE3Program,
    IComputeProvider
} from "./interfaces/IEnclave.sol";
import { ICiphernodeRegistry } from "./interfaces/ICiphernodeRegistry.sol";
import { IInputValidator } from "./interfaces/IInputValidator.sol";
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

    // TODO: should computation and compute providers be explicitly allowed?
    // My intuition is that an allowlist is required since they impose slashing conditions.
    // But perhaps this is one place where node pools might be utilized, allowing nodes to
    // opt in to being selected for specific computations, along with the corresponding slashing conditions.
    // This would reduce the governance overhead for Enclave.

    // Mapping of allowed E3 Programs.
    mapping(IE3Program e3Program => bool allowed) public e3Programs;

    // Mapping of allowed compute providers.
    mapping(IComputeProvider computeProvider => bool allowed)
        public computeProviders;

    // Mapping of E3s.
    mapping(uint256 e3Id => E3 e3) public e3s;

    // Mapping of input merkle trees
    mapping(uint256 e3Id => LeanIMTData imt) public inputs;

    // Mapping counting the number of inputs for each E3.
    mapping(uint256 e3Id => uint256 inputCount) public inputCounts;

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
    error InputDeadlinePassed(uint256 e3Id, uint256 expiration);
    error InputDeadlineNotPassed(uint256 e3Id, uint256 expiration);
    error InvalidComputation();
    error InvalidComputeProviderSetup();
    error InvalidCiphernodeRegistry(ICiphernodeRegistry ciphernodeRegistry);
    error InvalidInput();
    error InvalidDuration(uint256 duration);
    error InvalidOutput(bytes output);
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
    constructor(
        address _owner,
        ICiphernodeRegistry _ciphernodeRegistry,
        uint256 _maxDuration
    ) {
        initialize(_owner, _ciphernodeRegistry, _maxDuration);
    }

    /// @param _owner The owner of this contract
    /// @param _maxDuration The maximum duration of a computation in seconds
    function initialize(
        address _owner,
        ICiphernodeRegistry _ciphernodeRegistry,
        uint256 _maxDuration
    ) public initializer {
        __Ownable_init(msg.sender);
        setMaxDuration(_maxDuration);
        setCiphernodeRegistry(_ciphernodeRegistry);
        if (_owner != owner()) transferOwnership(_owner);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                  Core Entrypoints                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    function request(
        address filter,
        uint32[2] calldata threshold,
        uint256[2] calldata startWindow,
        uint256 duration,
        IE3Program e3Program,
        bytes memory e3ProgramParams,
        IComputeProvider computeProvider,
        bytes memory emParams
    ) external payable returns (uint256 e3Id, E3 memory e3) {
        // TODO: allow for other payment methods or only native tokens?
        // TODO: should payment checks be somewhere else? Perhaps in the E3 Program or ciphernode registry?
        require(msg.value > 0, PaymentRequired(msg.value));
        require(
            threshold[1] >= threshold[0] && threshold[0] > 0,
            InvalidThreshold(threshold)
        );
        require(
            // TODO: do we need a minimum start window to allow time for committee selection?
            startWindow[1] >= startWindow[0] &&
                startWindow[1] >= block.timestamp,
            InvalidStartWindow()
        );
        require(
            duration > 0 && duration <= maxDuration,
            InvalidDuration(duration)
        );
        require(e3Programs[e3Program], E3ProgramNotAllowed(e3Program));
        require(
            computeProviders[computeProvider],
            ModuleNotEnabled(address(computeProvider))
        );

        // TODO: should IDs be incremental or produced deterministically?
        e3Id = nexte3Id;
        nexte3Id++;

        IInputValidator inputValidator = e3Program.validate(e3ProgramParams);
        require(address(inputValidator) != address(0), InvalidComputation());

        // TODO: validate that the requested computation can be performed by the given compute provider.
        // Perhaps the compute provider should be returned by the E3 Program?
        IDecryptionVerifier decryptionVerifier = computeProvider.validate(
            emParams
        );
        require(
            address(decryptionVerifier) != address(0),
            InvalidComputeProviderSetup()
        );

        e3 = E3({
            threshold: threshold,
            startWindow: startWindow,
            duration: duration,
            expiration: 0,
            e3Program: e3Program,
            computeProvider: computeProvider,
            inputValidator: inputValidator,
            decryptionVerifier: decryptionVerifier,
            committeePublicKey: hex"",
            ciphertextOutput: hex"",
            plaintextOutput: hex""
        });
        e3s[e3Id] = e3;

        require(
            ciphernodeRegistry.requestCommittee(e3Id, filter, threshold),
            CommitteeSelectionFailed()
        );

        emit E3Requested(e3Id, e3s[e3Id], filter, e3Program, computeProvider);
    }

    function activate(uint256 e3Id) external returns (bool success) {
        // Note: we could load this into a storage pointer, and do the sets there
        // Requires a mew internal _getter that returns storage
        E3 memory e3 = getE3(e3Id);
        require(e3.expiration == 0, E3AlreadyActivated(e3Id));
        require(e3.startWindow[0] <= block.timestamp, E3NotReady());
        // TODO: handle what happens to the payment if the start window has passed.
        require(e3.startWindow[1] >= block.timestamp, E3Expired());

        bytes memory publicKey = ciphernodeRegistry.committeePublicKey(e3Id);
        // Note: This check feels weird
        require(publicKey.length > 0, CommitteeSelectionFailed());

        e3s[e3Id].expiration = block.timestamp + e3.duration;
        e3s[e3Id].committeePublicKey = publicKey;

        emit E3Activated(e3Id, e3.expiration, e3.committeePublicKey);

        return true;
    }

    function publishInput(
        uint256 e3Id,
        bytes memory data
    ) external returns (bool success) {
        E3 memory e3 = getE3(e3Id);

        // Note: if we make 0 a no expiration, this has to be refactored
        require(e3.expiration > 0, E3NotActivated(e3Id));
        // TODO: should we have an input window, including both a start and end timestamp?
        require(
            e3.expiration > block.timestamp,
            InputDeadlinePassed(e3Id, e3.expiration)
        );
        bytes memory input;
        (input, success) = e3.inputValidator.validate(msg.sender, data);
        require(success, InvalidInput());
        uint256 inputHash = PoseidonT3.hash(
            [uint256(keccak256(input)), inputCounts[e3Id]]
        );

        inputCounts[e3Id]++;
        inputs[e3Id]._insert(inputHash);

        emit InputPublished(e3Id, input, inputHash, inputCounts[e3Id] - 1);
    }

    function publishCiphertextOutput(
        uint256 e3Id,
        bytes memory data
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
            e3.ciphertextOutput.length == 0,
            CiphertextOutputAlreadyPublished(e3Id)
        );
        bytes memory output;
        (output, success) = e3.e3Program.verify(e3Id, data);
        require(success, InvalidOutput(output));
        e3s[e3Id].ciphertextOutput = output;

        emit CiphertextOutputPublished(e3Id, output);
    }

    function publishPlaintextOutput(
        uint256 e3Id,
        bytes memory data
    ) external returns (bool success) {
        E3 memory e3 = getE3(e3Id);
        // Note: if we make 0 a no expiration, this has to be refactored
        require(e3.expiration > 0, E3NotActivated(e3Id));
        require(
            e3.ciphertextOutput.length > 0,
            CiphertextOutputNotPublished(e3Id)
        );
        require(
            e3.plaintextOutput.length == 0,
            PlaintextOutputAlreadyPublished(e3Id)
        );
        bytes memory output;
        (output, success) = e3.decryptionVerifier.verify(e3Id, data);
        require(success, InvalidOutput(output));
        e3s[e3Id].plaintextOutput = output;

        emit PlaintextOutputPublished(e3Id, output);
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

    function enableComputeProvider(
        IComputeProvider computeProvider
    ) public onlyOwner returns (bool success) {
        require(
            !computeProviders[computeProvider],
            ModuleAlreadyEnabled(address(computeProvider))
        );
        computeProviders[computeProvider] = true;
        success = true;
        emit ComputeProviderEnabled(computeProvider);
    }

    function disableE3Program(
        IE3Program e3Program
    ) public onlyOwner returns (bool success) {
        require(e3Programs[e3Program], ModuleNotEnabled(address(e3Program)));
        delete e3Programs[e3Program];
        success = true;
        emit E3ProgramDisabled(e3Program);
    }

    function disableComputeProvider(
        IComputeProvider computeProvider
    ) public onlyOwner returns (bool success) {
        require(
            computeProviders[computeProvider],
            ModuleNotEnabled(address(computeProvider))
        );
        delete computeProviders[computeProvider];
        success = true;
        emit ComputeProviderDisabled(computeProvider);
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
}
