// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Threshold share decryption circuit.
//!
//! Proves one partial decryption share. The share is `λ·c1·sh + e_fresh + mask`.
//! `λ` is the centered Lagrange coefficient. `e_fresh` is one signed coefficient reduced
//! into each CRT limb. The witness produces Prover.toml and configs.nr for the Noir prover.

pub mod circuit;
pub mod codegen;
pub mod computation;
pub mod sample;
pub use circuit::*;
pub use codegen::*;
pub use computation::*;
