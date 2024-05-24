// SPDX-License-Identifier: LGPL-3.0-only
pragma solidity >=0.8.26;

import { IEnclave } from "./Interfaces/IEnclave.sol";
import { IComputationModule } from "./Interfaces/IComputationModule.sol";
import { ICypherNodeRegistry } from "./Interfaces/ICypherNodeRegistry.sol";
import { IExecutionModule } from "./Interfaces/IExecutionModule.sol";
import { IInputVerifier } from "./Interfaces/IInputVerifier.sol";

struct E3 {
    uint32[2] threshold; // M/N threshold for the committee.
    uint256 expiration; // timestamp when committee duties expire.
    address inputVerifier; // address of the input verifier contract.
    bytes32 committeeId; // ID of the selected committee.
}

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
        address indexed computationModule,
        address indexed executionModule
    );
    event InputPublished(uint256 e3Id, bytes data);

    error InputDeadlinePassed(uint e3Id, uint expiration);
    error InvalidComputation();
    error InvalidExecutionModuleSetup();
    error InvalidInput();
    error InvalidDuration();
    error InvalidThreshold();
    error PaymentRequired();

    constructor(uint256 _maxDuration) {
        maxDuration = _maxDuration;
    }

    function request(
        uint256 poolId,
        uint32[2] calldata threshold,
        uint256 duration,
        address computationModule,
        bytes memory computationParams,
        address executionModule,
        bytes memory emParams
    ) external payable returns (uint e3Id) {
        require(msg.value > 0, PaymentRequired()); // TODO: allow for other payment methods or only native tokens?

        require(threshold[1] >= threshold[0] && threshold[0] > 0, InvalidThreshold());
        require(duration > 0 && duration <= maxDuration, InvalidDuration());

        address inputVerifier = IComputationModule(computationModule).validate(computationParams);
        require(inputVerifier != address(0), InvalidComputation());

        // TODO: validate that the requested computation can be performed by the given execution module.
        require(IExecutionModule(executionModule).validate(emParams), InvalidExecutionModuleSetup());

        e3Id = nexte3Id;
        bytes32 committeeId = cypherNodeRegistry.selectCommittee(poolId, threshold);
        // TODO: validate that the selected pool accepts both the computation and execution modules.
        e3s[e3Id] = E3(threshold, block.timestamp + duration, inputVerifier, committeeId);

        emit E3Requested(e3Id, e3s[e3Id], poolId, computationModule, executionModule);
    }

    function input(uint e3Id, bytes calldata data) external {
        E3 storage e3 = e3s[e3Id];
        require(e3.expiration > block.timestamp, InputDeadlinePassed(e3Id, e3.expiration));
        require(IInputVerifier(e3.inputVerifier).validate(msg.sender, data), InvalidInput());

        emit InputPublished(e3Id, data);
    }
}
