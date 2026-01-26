# Parity Matrix Library

A library for generating parity matrices for linear subspaces of Z_q^{n+1} consisting of polynomial
evaluations. Designed for cryptographic applications, particularly in homomorphic encryption and
error-correcting codes.

## Features

- Generator matrix construction for polynomial evaluation subspaces
- Null space computation using Gaussian elimination over finite fields
- Parity matrix verification
- Support for arbitrary modulus q using `num-bigint`
- Command-line tool for interactive matrix generation

## Mathematical Background

This library generates parity matrices for the linear subspace of Z_q^{n+1} consisting of polynomial
evaluations of degree at most t at points 0, 1, ..., n.

### Generator Matrix

The generator matrix G has dimensions (t+1) × (n+1), where:

- Each row i corresponds to evaluations of x^i at points 0, 1, ..., n
- G[i][j] = j^i mod q
- For polynomials of degree t, we have t+1 coefficients (a_0, ..., a_t)

### Parity Matrix

The parity matrix H is the null space of the generator matrix G, satisfying:

- H · G^T = 0 (mod q)
- Dimensions: (n+1 - (t+1)) × (n+1)

### Constraint

The degree t must satisfy: **t ≤ (n-1)/2**

This ensures the subspace has a non-trivial parity check matrix.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
parity-matrix = { git = "https://github.com/gnosisguild/enclave", path = "crates/parity_matrix" }
```

## Usage

### Library Usage

```rust
use parity_matrix::matrix::{ParityMatrixConfig, build_generator_matrix, null_space, verify_parity_matrix};
use num_bigint::BigUint;

// Configure parameters
let config = ParityMatrixConfig {
    q: BigUint::from(101u128),  // Modulus
    t: 4,                        // Polynomial degree
    n: 10,                       // Number of points
};

// Build generator matrix
let g = build_generator_matrix(config.clone())?;

// Compute parity matrix (null space)
let h = null_space(&g, &config.q)?;

// Verify correctness
let is_valid = verify_parity_matrix(&g, &h, &config.q)?;
assert!(is_valid);
```

### Command-Line Tool

The crate includes a binary for interactive matrix generation:

```bash
cargo run --bin parity-matrix -- --q 101 --n 10 --t 4
```

With verbose output:

```bash
cargo run --bin parity-matrix -- --q 101 --n 10 --t 4 --verbose
```

## Testing

Run the test suite:

```bash
cargo test
```

Run tests with verbose output:

```bash
cargo test -- --nocapture
```
