// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! DKG public-key BFV commitment circuit.
//!
//! This circuit proves knowledge of a DKG BFV public key (pk0, pk1) and produces
//! Prover.toml and configs.nr for the Noir prover. See [`PkCircuit`] and
//! [`PkCircuitInput`].

pub mod circuit;
pub mod codegen;
pub mod computation;
pub use circuit::*;
pub use codegen::*;
pub use computation::*;
