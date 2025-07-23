<div align="center">
  <picture>
    <img src="./enclave.png" alt="Enclave" width="100%">
  </picture>

[![Docs][docs-badge]][docs] [![Github Actions][gha-badge]][gha] [![Hardhat][hardhat-badge]][hardhat] [![License: LGPL v3][license-badge]][license]

</div>

# Enclave

This is the monorepo for Enclave, an open-source protocol for Collaborative Confidential Compute. Enclave leverages the combination of Fully Homomorphic Encryption (FHE), Zero Knowledge Proofs (ZKPs), and Multi-Party Computation (MPC) to enable Encrypted Execution Environments (E3) with integrity and privacy guarantees rooted in cryptography and economics, rather than hardware and attestations.

## Quick Start

Follow instructions in the [quick start][quick-start] section of the [Enclave docs][docs].

See the [CRISP example][crisp] for a fully functioning example application.

## Getting Help

Join the Enclave [Telegram group][telegram].

## Contributing

See [CONTRIBUTING.md][CONTRIBUTING].

## Minimum Rust version

This workspace's minimum supported rustc version is 1.86.0.

## Architecture

Enclave employs a modular architecture involving numerous actors and participants. The sequence diagram below offers a high-level overview of the protocol, but necessarily omits most detail.

```mermaid
sequenceDiagram
    participant Users
    participant Enclave
    participant CiphernodeRegistry
    participant E3Program
    participant ComputeProvider
    participant InputValidator
    participant DecryptionVerifier

    Users->>Enclave: request(parameters)
    Enclave->>E3Program: validate(e3ProgramParams)
    E3Program-->>Enclave: inputValidator
    Enclave->>ComputeProvider: validate(computeProviderParams)
    ComputeProvider-->>Enclave: decryptionVerifier
    Enclave->>CiphernodeRegistry: requestCommittee(e3Id, filter, threshold)
    CiphernodeRegistry-->>Enclave: success
    Enclave-->>Users: e3Id, E3 struct

    Users->>Enclave: activate(e3Id)
    Enclave->>CiphernodeRegistry: committeePublicKey(e3Id)
    CiphernodeRegistry-->>Enclave: publicKey
    Enclave->>Enclave: Set expiration and committeePublicKey
    Enclave-->>Users: success

    Users->>Enclave: publishInput(e3Id, data)
    Enclave->>InputValidator: validate(msg.sender, data)
    InputValidator-->>Enclave: input, success
    Enclave->>Enclave: Store input
    Enclave-->>Users: success

    Users->>Enclave: publishCiphertextOutput(e3Id, data)
    Enclave->>DecryptionVerifier: verify(e3Id, data)
    DecryptionVerifier-->>Enclave: output, success
    Enclave->>Enclave: Store ciphertextOutput
    Enclave-->>Users: success

    Users->>Enclave: publishPlaintextOutput(e3Id, data)
    Enclave->>E3Program: verify(e3Id, data)
    E3Program-->>Enclave: output, success
    Enclave->>Enclave: Store plaintextOutput
    Enclave-->>Users: success
```

## Security and Liability

This repo is provided WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.

## License

This repo created under the [LGPL-3.0+ license](LICENSE).

[gha]: https://github.com/gnosisguild/enclave/actions
[gha-badge]: https://github.com/gnosisguild/enclave/actions/workflows/ci.yml/badge.svg
[hardhat]: https://hardhat.org/
[hardhat-badge]: https://img.shields.io/badge/Built%20with-Hardhat-FFDB1C.svg
[license]: https://opensource.org/license/lgpl-3-0
[license-badge]: https://img.shields.io/badge/License-LGPLv3.0-blue.svg
[docs]: https://docs.enclave.gg
[docs-badge]: https://img.shields.io/badge/Documentation-blue.svg
[quick-start]: https://docs.enclave.gg/quick-start
[crisp]: https://docs.enclave.gg/CRISP/introduction
[telegram]: https://t.me/+raYAZgrwgOw2ODJh
