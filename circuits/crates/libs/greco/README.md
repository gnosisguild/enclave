# Greco

This package contains a zero-knowledge proof circuit in Noir for verifying the correct formation of ciphertexts resulting from BFV (Brakerski-Fan-Vercauteren) public key encryption.

- Proves correct ciphertext formation without revealing secrets
- Implements the Brakerski-Fan-Vercauteren homomorphic encryption scheme
- Supports multiple Chinese Remainder Theorem (CRT) bases for efficient computation
- Comprehensive bounds validation for all polynomial coefficients
- Uses the Fiat-Shamir heuristic for non-interactive proof generation
- Leverages polynomial identity testing for efficient verification

## Overview

The Greco circuit verifies that ciphertext components `(ct0, ct1)` are correctly computed from public key components `(pk0, pk1)` and encryption randomness. The circuit enforces:

1. **Range Constraints**: All polynomial coefficients must be within expected bounds
2. **Encryption Equations**:
   - `ct0i(γ) = pk0i(γ) * u(γ) + e0(γ) + k1(γ) * k0i + r1i(γ) * qi + r2i(γ) * cyclo(γ)`
   - `ct1i(γ) = pk1i(γ) * u(γ) + e1(γ) + p1i(γ) * qi + p2i(γ) * cyclo(γ)`

Where `cyclo(γ) = γ^N + 1` is the cyclotomic polynomial.

## Installation

In your _Nargo.toml_ file, add this library as a dependency:

```toml
[dependencies]
greco = { tag = "v0.1.0", git = "https://github.com/gnosisguild/enclave", directory = "circuits/crates/libs/greco"}
```

## API Reference

### Core Structures

#### `Params<N, L>`

Complete parameters combining cryptographic and bound parameters.

#### `CryptographicParams<L>`

Contains core mathematical constants:

- `q_mod_t`: Plaintext modulus
- `qis`: CRT moduli for each basis
- `k0is`: Scaling factors for each basis

#### `BoundParams<L>`

Contains all bounds for range checking:

- `pk_bounds`: Public key polynomial bounds
- `e_bound`: Error polynomial bound
- `u_bound`: Secret polynomial bound
- `r1_low_bounds`, `r1_up_bounds`: Modulus switching bounds
- `r2_bounds`: Cyclotomic reduction bounds
- `p1_bounds`, `p2_bounds`: Additional randomness bounds
- `k1_low_bound`, `k1_up_bound`: Scaled message bounds

#### `Greco<N, L>`

Main circuit structure implementing the zero-knowledge proof.

### Key Methods

- `new()`: Creates a new Greco circuit instance
- `verify_correct_ciphertext_encryption()`: Performs the complete verification
- `check_range_bounds()`: Validates all coefficient bounds
- `generate_challenge()`: Generates Fiat-Shamir challenges
- `check_encryption_constraints()`: Verifies encryption equations

## Generic Parameters

- `N`: Polynomial degree (ring dimension)
- `L`: Number of CRT bases

## Disclaimer

This circuit is a port of the Halo2 implementation from the Greco paper authors at PSE. The original Halo2 implementation is available at [https://github.com/privacy-scaling-explorations/greco](https://github.com/privacy-scaling-explorations/greco). We extend our gratitude for their groundbreaking work on zero-knowledge proofs for BFV encryption correctness, detailed in their [research paper](https://eprint.iacr.org/2024/594).

## Compatibility

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
