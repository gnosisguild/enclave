// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { IEnclave, E3, IComputationModule, IExecutionModule } from "./Interfaces/IEnclave.sol";
import { ICypherNodeRegistry } from "./Interfaces/ICypherNodeRegistry.sol";
import { IInputValidator } from "./Interfaces/IInputValidator.sol";

contract Enclave is IEnclave {
    ICypherNodeRegistry public cypherNodeRegistry; // TODO: add a setter function.
    uint256 public maxDuration; // TODO: add a setter function.
    uint256 public nexte3Id; // ID of the next E3.
    uint256 public requests; // total number of requests made to Enclave.

    mapping(address moduleAddress => bool allowed) public computationModules; // Mapping of allowed computation modules.
    mapping(address moduleAddress => bool allowed) public executionModules; // Mapping of allowed execution modules.
    mapping(uint256 id => E3) public e3s; // Mapping of E3s.

    event E3Requested(
        uint256 e3Id,
        E3 e3,
        uint256 indexed poolId,
        IComputationModule indexed computationModule,
        IExecutionModule indexed executionModule
    );
    event InputPublished(uint256 e3Id, bytes data);

    error InputDeadlinePassed(uint e3Id, uint expiration);
    error InvalidComputation();
    error InvalidExecutionModuleSetup();
    error InvalidInput();
    error InvalidDuration();
    error InvalidThreshold();
    error PaymentRequired();

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
    ) external payable returns (uint e3Id, E3 memory e3) {
        require(msg.value > 0, PaymentRequired()); // TODO: allow for other payment methods or only native tokens?

        require(threshold[1] >= threshold[0] && threshold[0] > 0, InvalidThreshold());
        require(duration > 0 && duration <= maxDuration, InvalidDuration());

        e3Id = nexte3Id;
        nexte3Id++;

        IInputValidator inputValidator = computationModule.validate(computationParams);
        require(address(inputValidator) != address(0), InvalidComputation());

        // TODO: validate that the requested computation can be performed by the given execution module.
        require(executionModule.validate(emParams), InvalidExecutionModuleSetup());

        bytes32 committeeId = cypherNodeRegistry.selectCommittee(poolId, threshold);
        // TODO: validate that the selected pool accepts both the computation and execution modules.

        e3 = E3({
            threshold: threshold,
            expiration: block.timestamp + duration,
            computationModule: computationModule,
            executionModule: executionModule,
            inputValidator: inputValidator,
            committeeId: committeeId
        });
        e3s[e3Id] = e3;

        emit E3Requested(e3Id, e3s[e3Id], poolId, computationModule, executionModule);
    }

    function input(uint e3Id, bytes calldata data) external returns (bool success) {
        E3 storage e3 = e3s[e3Id];
        require(e3.expiration > block.timestamp, InputDeadlinePassed(e3Id, e3.expiration));
        success = e3.inputValidator.validate(msg.sender, data);
        require(success, InvalidInput());

        emit InputPublished(e3Id, data);
    }
}
