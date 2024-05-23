# Enclave Specification

This document is a specification of the smart contract components to Enclave, an open source protocol for Encrypted Execution Environments (E3).

## Actors

There are five groups of actors in Enclave:

1. **Requesters:** Anyone can request an E3 from the Enclave protocol by calling the corresponding smart contract entrypoint and depositing a bond proportional to the number, threshold, and duration of Cypher Nodes that they request.
2. **Data Providers:** Individuals and systems providing inputs to a requested E3. Data Providers contribute data encrypted to the public threshold key that is created, and published on chain, by the Cypher Nodes selected for a requested E3.
3. **Execution Modules:** Enclave is a modular framework, allowing the choice of many different Execution Modules in which to run encrypted computations. Broadly, Execution Modules fall into two categories: (1) Provable (like RISC Zero’s virtual machine[^1], Arbitrum’s WAVM[^2], or Succinct's SP1[^3]) and (2) Oracle-based. The former provides cryptographic guarantees of correct execution, while the latter provides economic guarantees of correct execution.
4. **Cypher Nodes:** Cypher Nodes are responsible for creating threshold public keys and decrypting the cyphertext output for each requested computation. Cypher Nodes can be registered by anyone staking Enclave tokens.
5. **Token Holders:** As the top-level governance body, Enclave token holders are responsible for setting protocol parameters, overseeing protocol upgrades, and facilitating dispute resolution.

Enclave is a smart contract protocol for coordinating the interactions between these various actors.

## Components

Enclave is a modular architecture, this section describes each of the various smart contract components that constitute the Enclave protocol.

### Core

Contains the main entrypoints for requesting and publishing inputs to E3s.

**`requestE3(uint256 computationId, bytes memory data)`**

**`publishInput(bytes32 e3Id, bytes memory data)`**

**`publishOutput(bytes32 e3Id, bytes memory data)`**

**`registerNode()`**

### CyphernodeRegistry

Registry of staked Cyphernodes that are eligible to be selected for E3 duties.

### ComputationRegistry

Registry of computations which can be requested via the protocol.

### IComputationModule

Computation module contracts implement any specific

### ExecutionModuleRegistry

Registry of execution modules on which a requested computation can be run.

### IExecutionModule

Interface defining interactions with any given execution module.

---

[^1]: RISC Zero is a general-purpose, zero-knowledge virtual machine. More information can be found on their website at https://risczero.com
[^2]: WAVM is Arbitrum’s execution environment, provable via optimistic fraud proofs. More information can be found on their website at https://arbitrum.io
[^3]: SP1 is a performant, 100% open-source, contributor-friendly zero-knowledge virtual machine (zkVM) that can prove the execution of arbitrary Rust (or any LLVM-compiled language) programs. More information can be found on Succinct's github at https://github.com/succinctlabs/sp1
