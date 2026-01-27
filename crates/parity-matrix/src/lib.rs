// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Parity matrix generation for polynomial evaluation verification.
//!
//! This crate generates parity check matrices for detecting errors in polynomial evaluations
//! over finite fields. Given a polynomial of degree `t` evaluated at `n+1` points modulo `q`,
//! it constructs a generator matrix `G` and its null space `H` such that `H · G^T = 0 (mod q)`.
//!
//! ## Mathematical Background
//!
//! For a polynomial `f(x) = a₀ + a₁x + ... + aₜxᵗ` of degree `t`, the generator matrix `G`
//! has dimensions `(t+1) × (n+1)` where `G[i][j] = j^i mod q`. The evaluation vector
//! `v = [f(0), f(1), ..., f(n)]` can be written as `v = G^T · [a₀, ..., aₜ]`.
//!
//! The parity check matrix `H` is a basis for the null space of `G`, satisfying `H · G^T = 0`.
//! Any valid polynomial evaluation vector `v` must satisfy `H · v = 0 (mod q)`, allowing
//! detection of errors or degree violations.
//!
//! ## Constraint
//!
//! The degree `t` must satisfy `t ≤ (n-1)/2` to ensure the system is well-defined.

pub mod errors;
pub mod math;
pub mod matrix;
pub mod matrix_type;
pub mod utils;

// Re-export commonly used types for convenience
pub use matrix_type::{DynamicMatrix, MatrixLike};
pub use errors::{ParityMatrixError, ParityMatrixResult};
pub use matrix::{ParityMatrixConfig, build_generator_matrix, null_space, verify_parity_matrix};
