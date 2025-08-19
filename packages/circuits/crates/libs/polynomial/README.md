# Polynomial Noir Library

This package contains a custom polynomial structure object and evaluation functions for use in zero-knowledge circuits.

- Efficient polynomial evaluation using Horner's method
- Cryptographic range checking for polynomial coefficients with symmetric and asymmetric bounds

## Installation

In your _Nargo.toml_ file, add this library as a dependency:

```toml
[dependencies]
polynomial = { tag = "v0.1.0", git = "https://github.com/gnosisguild/enclave", directory = "packages/circuits/libs/polynomial"}
```

### Compatibility

This has been developed and tested with

```bash
nargo version = 1.0.0-beta.3
noirc version = 1.0.0-beta.3+ceaa1986628197bd1170147f6a07f0f98d21030a
(git version hash: ceaa1986628197bd1170147f6a07f0f98d21030a, is dirty: false)
```
