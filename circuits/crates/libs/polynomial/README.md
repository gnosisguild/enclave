# Polynomial Noir Library

This package contains a custom polynomial structure object and evaluation functions for use in
zero-knowledge circuits.

- Efficient polynomial evaluation using Horner's method
- Cryptographic range checking for polynomial coefficients with symmetric and asymmetric bounds

## Installation

In your _Nargo.toml_ file, add this library as a dependency:

```toml
[dependencies]
polynomial = { tag = "v0.1.5", git = "https://github.com/gnosisguild/enclave", directory = "packages/circuits/crates/libs/polynomial"}
```

nb. the `tag` corresponds to the latest tag release of Enclave (`v0.1.5`). From `v0.1.6` you should
remove `packages/` from `directory` field (ie., `circuits/crates/...`).

### Compatibility

This has been developed and tested with

```bash
nargo --version
nargo version = 1.0.0-beta.15
noirc version = 1.0.0-beta.15+83245db91dcf63420ef4bcbbd85b98f397fee663
(git version hash: 83245db91dcf63420ef4bcbbd85b98f397fee663, is dirty: false)
```

```bash
bb --version
3.0.0-nightly.20251104
```
