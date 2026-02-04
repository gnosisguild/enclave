// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Zero-knowledge circuit types and code generation.
//!
//! This module provides circuit metadata ([`Circuit`](crate::registry::Circuit)), artifact
//! codegen ([`CircuitCodegen`], [`Artifacts`]), commitment helpers ([`commitments`]),
//! and sample data generation ([`Sample`]). The `pk_bfv` submodule implements the
//! public-key BFV commitment circuit.

pub mod codegen;
pub mod commitments;
pub mod computation;
pub mod errors;
pub mod sample;

pub use codegen::{write_artifacts, Artifacts, CircuitCodegen};
pub use commitments::*;
pub use computation::{
    CircuitComputation, Computation, Configs, ConvertToJson, ReduceToZkpModulus, Toml,
};
pub use errors::CircuitsErrors;
pub use sample::Sample;

pub mod pk_bfv;
pub use pk_bfv::codegen::{generate_configs, generate_toml, TomlJson};
pub use pk_bfv::computation::{Bits, Bounds, PkBfvComputationOutput, Witness};
pub use pk_bfv::PkBfvCircuit;
