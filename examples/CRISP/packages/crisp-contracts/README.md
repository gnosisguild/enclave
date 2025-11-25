# Solidity Contracts

This directory contains the Solidity contracts for CRISP - Coercion-Resistant Impartial Selection
Protocol.

Contracts are built and tested with [Hardhat](https://hardhat.org). Tests are defined in the `test` directory.

## Running Tests

To run contract tests from the CRISP example root (`examples/CRISP/`):

```bash
pnpm test:contracts
```

Alternatively, you can run tests directly from this directory:

```bash
pnpm test
```

## CRISP Program

This is the main logic of CRISP - an enclave program for secure voting.

It exposes two main functions:

- `validate` - that is called when a new E3 instance is requested on Enclave (`Enclave.request`).
- `verify` - that is called when the ciphertext output is published on Enclave
  (`Enclave.publishCiphertextOutput`). This function ensures that the ciphertext output is valid.
  CRISP uses Risc0 as the compute provider for running the FHE program, thus the proof will be a
  Risc0 proof.

## Input validator

The input validator contract is used to validate the input data that is submitted to the E3
instance. It is called by the Enclave contract when a new input is published
(`Enclave.publishInput`). In CRISP, the data providers (the ones submitting the inputs) are the
voters, and the input submitted is the vote itself.

The validator checks that gating conditions are satisfied and that the ciphertext is constructed
correctly using
[Greco](https://github.com/gnosisguild/enclave/tree/main/circuits/crates/libs/greco). See the Greco
[paper](https://eprint.iacr.org/2024/594).
