// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { IEnclave, E3, IComputationModule, IExecutionModule } from "./Interfaces/IEnclave.sol";
import { ICypherNodeRegistry } from "./Interfaces/ICypherNodeRegistry.sol";
import { IInputValidator } from "./Interfaces/IInputValidator.sol";
import { IOutputVerifier } from "./Interfaces/IOutputVerifier.sol";

contract Enclave is IEnclave {
    ICypherNodeRegistry public cypherNodeRegistry; // TODO: add a setter function.
    uint256 public maxDuration; // TODO: add a setter function.
    uint256 public nexte3Id; // ID of the next E3.
    uint256 public requests; // total number of requests made to Enclave.

    mapping(address moduleAddress => bool allowed) public computationModules; // Mapping of allowed computation modules.
    mapping(address moduleAddress => bool allowed) public executionModules; // Mapping of allowed execution modules.
    mapping(uint256 id => E3) public e3s; // Mapping of E3s.

    event CiphertextOutputPublished(uint256 e3Id, bytes ciphertextOutput);
    event E3Requested(
        uint256 e3Id,
        E3 e3,
        uint256 indexed poolId,
        IComputationModule indexed computationModule,
        IExecutionModule indexed executionModule
    );
    event InputPublished(uint256 e3Id, bytes data);
    event PlaintextOutputPublished(uint256 e3Id, bytes plaintextOutput);

    error InputDeadlinePassed(uint256 e3Id, uint256 expiration);
    error InputDeadlineNotPassed(uint256 e3Id, uint256 expiration);
    error InvalidComputation();
    error InvalidExecutionModuleSetup();
    error InvalidInput();
    error InvalidDuration();
    error InvalidOutput();
    error InvalidThreshold();
    error CiphertextOutputAlreadyPublished(uint256 e3Id);
    error CiphertextOutputNotPublished(uint256 e3Id);
    error PaymentRequired();
    error PlaintextOutputAlreadyPublished(uint256 e3Id);

    /// @param _maxDuration The maximum duration of a computation in seconds.
    constructor(uint256 _maxDuration) {
        maxDuration = _maxDuration;
    }

    function request(
        uint256 poolId,
        uint32[2] calldata threshold,
        uint256 duration,
        IComputationModule computationModule,
        bytes memory computationParams,
        IExecutionModule executionModule,
        bytes memory emParams
    ) external payable returns (uint256 e3Id, E3 memory e3) {
        require(msg.value > 0, PaymentRequired()); // TODO: allow for other payment methods or only native tokens?

        require(threshold[1] >= threshold[0] && threshold[0] > 0, InvalidThreshold());
        require(duration > 0 && duration <= maxDuration, InvalidDuration());

        e3Id = nexte3Id;
        nexte3Id++;

        IInputValidator inputValidator = computationModule.validate(computationParams);
        require(address(inputValidator) != address(0), InvalidComputation());

        // TODO: validate that the requested computation can be performed by the given execution module.
        IOutputVerifier outputVerifier = executionModule.validate(emParams);
        require(address(outputVerifier) != address(0), InvalidExecutionModuleSetup());

        bytes32 committeeId = cypherNodeRegistry.selectCommittee(poolId, threshold);
        // TODO: validate that the selected pool accepts both the computation and execution modules.

        e3 = E3({
            threshold: threshold,
            expiration: block.timestamp + duration,
            computationModule: computationModule,
            executionModule: executionModule,
            inputValidator: inputValidator,
            outputVerifier: outputVerifier,
            committeeId: committeeId,
            ciphertextOutput: new bytes(0),
            plaintextOutput: new bytes(0)
        });
        e3s[e3Id] = e3;

        emit E3Requested(e3Id, e3s[e3Id], poolId, computationModule, executionModule);
    }

    function publishInput(uint256 e3Id, bytes memory data) external returns (bool success) {
        E3 storage e3 = e3s[e3Id];
        require(e3.expiration > block.timestamp, InputDeadlinePassed(e3Id, e3.expiration));
        bytes memory input;
        (input, success) = e3.inputValidator.validate(msg.sender, data);
        require(success, InvalidInput());

        emit InputPublished(e3Id, input);
    }

    function publishOutput(uint256 e3Id, bytes memory data) external returns (bool success) {
        E3 storage e3 = e3s[e3Id];
        require(e3.expiration <= block.timestamp, InputDeadlineNotPassed(e3Id, e3.expiration));
        require(e3.ciphertextOutput.length == 0, CiphertextOutputAlreadyPublished(e3Id));
        bytes memory output;
        (output, success) = e3.outputVerifier.verify(e3Id, data);
        require(success, InvalidOutput());
        e3.ciphertextOutput = output;

        emit CiphertextOutputPublished(e3Id, output);
    }

    function decryptOutput(uint256 e3Id, bytes memory data) external returns (bool success) {
        E3 storage e3 = e3s[e3Id];
        require(e3.ciphertextOutput.length > 0, CiphertextOutputNotPublished(e3Id));
        require(e3.plaintextOutput.length == 0, PlaintextOutputAlreadyPublished(e3Id));
        bytes memory output;
        (output, success) = e3.computationModule.verify(e3Id, data);
        e3.plaintextOutput = output;

        emit PlaintextOutputPublished(e3Id, output);
    }
}
