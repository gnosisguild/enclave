// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import {
    IEnclave,
    E3,
    IComputationModule,
    IExecutionModule
} from "./interfaces/IEnclave.sol";
import { ICyphernodeRegistry } from "./interfaces/ICyphernodeRegistry.sol";
import { IInputValidator } from "./interfaces/IInputValidator.sol";
import { IOutputVerifier } from "./interfaces/IOutputVerifier.sol";
import {
    OwnableUpgradeable
} from "@openzeppelin/contracts-upgradeable/access/OwnableUpgradeable.sol";

contract Enclave is IEnclave, OwnableUpgradeable {
    ////////////////////////////////////////////////////////////
    //                                                        //
    //                 Storage Variables                      //
    //                                                        //
    ////////////////////////////////////////////////////////////

    ICyphernodeRegistry public cyphernodeRegistry; // TODO: add a setter function.
    uint256 public maxDuration; // TODO: add a setter function.
    uint256 public nexte3Id; // ID of the next E3.
    uint256 public requests; // total number of requests made to Enclave.

    // TODO: should computation and execution modules be explicitly allowed?
    // My intuition is that an allowlist is required since they impose slashing conditions.
    // But perhaps this is one place where node pools might be utilized, allowing nodes to
    // opt in to being selected for specific computations, along with the corresponding slashing conditions.
    // This would reduce the governance overhead for Enclave.

    // Mapping of allowed computation modules.
    mapping(IComputationModule computationModule => bool allowed)
        public computationModules;

    // Mapping of allowed execution modules.
    mapping(IExecutionModule executionModule => bool allowed)
        public executionModules;

    // Mapping of E3s.
    mapping(uint256 id => E3 e3) public e3s;

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                        Errors                          //
    //                                                        //
    ////////////////////////////////////////////////////////////

    error CommitteeSelectionFailed();
    error ComputationModuleNotAllowed(IComputationModule computationModule);
    error E3AlreadyActivated(uint256 e3Id);
    error E3NotActivated(uint256 e3Id);
    error E3DoesNotExist(uint256 e3Id);
    error ModuleAlreadyEnabled(address module);
    error ModuleNotEnabled(address module);
    error InputDeadlinePassed(uint256 e3Id, uint256 expiration);
    error InputDeadlineNotPassed(uint256 e3Id, uint256 expiration);
    error InvalidComputation();
    error InvalidExecutionModuleSetup();
    error InvalidCyphernodeRegistry(ICyphernodeRegistry cyphernodeRegistry);
    error InvalidInput();
    error InvalidDuration(uint256 duration);
    error InvalidOutput();
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
        ICyphernodeRegistry _cyphernodeRegistry,
        uint256 _maxDuration
    ) {
        initialize(_owner, _cyphernodeRegistry, _maxDuration);
    }

    /// @param _owner The owner of this contract
    /// @param _maxDuration The maximum duration of a computation in seconds
    function initialize(
        address _owner,
        ICyphernodeRegistry _cyphernodeRegistry,
        uint256 _maxDuration
    ) public initializer {
        __Ownable_init(msg.sender);
        setMaxDuration(_maxDuration);
        setCyphernodeRegistry(_cyphernodeRegistry);
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
        // TODO: do we also need a start block/time? Would it be possible to have computations where inputs are
        //published before the request is made? This kind of assumes the cypher nodes have already been selected
        // and generated a shared secret.
        uint256 duration,
        IComputationModule computationModule,
        bytes memory computationParams,
        IExecutionModule executionModule,
        bytes memory emParams
    ) external payable returns (uint256 e3Id, E3 memory e3) {
        // TODO: allow for other payment methods or only native tokens?
        // TODO: should payment checks be somewhere else? Perhaps in the computation module or cyphernode registry?
        require(msg.value > 0, PaymentRequired(msg.value));

        require(
            threshold[1] >= threshold[0] && threshold[0] > 0,
            InvalidThreshold(threshold)
        );
        // TODO: should 0 be a magic number for infinite duration?
        require(
            duration > 0 && duration <= maxDuration,
            InvalidDuration(duration)
        );
        require(
            computationModules[computationModule],
            ComputationModuleNotAllowed(computationModule)
        );
        require(
            executionModules[executionModule],
            ModuleNotEnabled(address(executionModule))
        );

        // TODO: should IDs be incremental or produced deterministic?
        e3Id = nexte3Id;
        nexte3Id++;

        IInputValidator inputValidator = computationModule.validate(
            computationParams
        );
        require(address(inputValidator) != address(0), InvalidComputation());

        // TODO: validate that the requested computation can be performed by the given execution module.
        IOutputVerifier outputVerifier = executionModule.validate(emParams);
        require(
            address(outputVerifier) != address(0),
            InvalidExecutionModuleSetup()
        );

        e3 = E3({
            threshold: threshold,
            expiration: 0,
            computationModule: computationModule,
            executionModule: executionModule,
            inputValidator: inputValidator,
            outputVerifier: outputVerifier,
            committeePublicKey: hex"",
            ciphertextOutput: hex"",
            plaintextOutput: hex""
        });
        e3s[e3Id] = e3;

        require(
            cyphernodeRegistry.requestCommittee(e3Id, filter, threshold),
            CommitteeSelectionFailed()
        );
        // TODO: validate that the selected pool accepts both the computation and execution modules.

        emit E3Requested(
            e3Id,
            e3s[e3Id],
            filter,
            computationModule,
            executionModule
        );
    }

    function activate(uint256 e3Id) external returns (bool success) {
        // Note: we could load this into a storage pointer, and do the sets there
        // Requires a mew internal _getter that returns storage
        E3 memory e3 = getE3(e3Id);
        require(e3.expiration == 0, E3AlreadyActivated(e3Id));

        bytes memory publicKey = cyphernodeRegistry.committeePublicKey(e3Id);
        // Note: This check feels weird
        require(publicKey.length > 0, CommitteeSelectionFailed());

        // TODO: this should be based on the duration requested, not the current max duration.
        e3s[e3Id].expiration = block.timestamp + maxDuration;
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
        // TODO: do we need to store or accumulate the inputs? Probably yes.
        emit InputPublished(e3Id, input);
    }

    function publishCiphertextOutput(
        uint256 e3Id,
        bytes memory data
    ) external returns (bool success) {
        E3 memory e3 = getE3(e3Id);
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
        (output, success) = e3.outputVerifier.verify(e3Id, data);
        require(success, InvalidOutput());
        e3s[e3Id].ciphertextOutput = output;

        emit CiphertextOutputPublished(e3Id, output);
    }

    function publishPlaintextOutput(
        uint256 e3Id,
        bytes memory data
    ) external returns (bool success) {
        E3 memory e3 = getE3(e3Id);
        require(
            e3.ciphertextOutput.length > 0,
            CiphertextOutputNotPublished(e3Id)
        );
        require(
            e3.plaintextOutput.length == 0,
            PlaintextOutputAlreadyPublished(e3Id)
        );
        bytes memory output;
        (output, success) = e3.computationModule.verify(e3Id, data);
        require(success, InvalidOutput());
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

    function setCyphernodeRegistry(
        ICyphernodeRegistry _cyphernodeRegistry
    ) public onlyOwner returns (bool success) {
        require(
            address(_cyphernodeRegistry) != address(0) &&
                _cyphernodeRegistry != cyphernodeRegistry,
            InvalidCyphernodeRegistry(_cyphernodeRegistry)
        );
        cyphernodeRegistry = _cyphernodeRegistry;
        success = true;
        emit CyphernodeRegistrySet(address(_cyphernodeRegistry));
    }

    function enableComputationModule(
        IComputationModule computationModule
    ) public onlyOwner returns (bool success) {
        require(
            !computationModules[computationModule],
            ModuleAlreadyEnabled(address(computationModule))
        );
        computationModules[computationModule] = true;
        success = true;
        emit ComputationModuleEnabled(computationModule);
    }

    function enableExecutionModule(
        IExecutionModule executionModule
    ) public onlyOwner returns (bool success) {
        require(
            !executionModules[executionModule],
            ModuleAlreadyEnabled(address(executionModule))
        );
        executionModules[executionModule] = true;
        success = true;
        emit ExecutionModuleEnabled(executionModule);
    }

    function disableComputationModule(
        IComputationModule computationModule
    ) public onlyOwner returns (bool success) {
        require(
            computationModules[computationModule],
            ModuleNotEnabled(address(computationModule))
        );
        delete computationModules[computationModule];
        success = true;
        emit ComputationModuleDisabled(computationModule);
    }

    function disableExecutionModule(
        IExecutionModule executionModule
    ) public onlyOwner returns (bool success) {
        require(
            executionModules[executionModule],
            ModuleNotEnabled(address(executionModule))
        );
        delete executionModules[executionModule];
        success = true;
        emit ExecutionModuleDisabled(executionModule);
    }

    ////////////////////////////////////////////////////////////
    //                                                        //
    //                   Get Functions                        //
    //                                                        //
    ////////////////////////////////////////////////////////////

    function getE3(uint256 e3Id) public view returns (E3 memory e3) {
        e3 = e3s[e3Id];
        require(
            e3.computationModule != IComputationModule(address(0)),
            E3DoesNotExist(e3Id)
        );
    }
}
