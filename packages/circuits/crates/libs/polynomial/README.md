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
nargo --version
nargo version = 1.0.0-beta.11
noirc version = 1.0.0-beta.11+fd3925aaaeb76c76319f44590d135498ef41ea6c
(git version hash: fd3925aaaeb76c76319f44590d135498ef41ea6c, is dirty: false)
```

```bash
bb --version
v0.87.0
```
