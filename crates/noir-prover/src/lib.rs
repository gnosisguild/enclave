// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod config;
mod error;
mod pkbfv;
mod prover;
mod setup;
mod witness;

pub use config::{NoirConfig, VersionInfo};
pub use error::NoirProverError;
pub use pkbfv::{prove_pk_bfv, verify_pk_bfv};
pub use prover::NoirProver;
pub use setup::{NoirSetup, SetupStatus};
pub use witness::{input_map, CompiledCircuit, WitnessGenerator};
