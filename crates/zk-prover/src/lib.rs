// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod backend;
mod circuits;
mod config;
mod error;
pub mod ext;
mod prover;
mod traits;
mod witness;

pub use backend::{SetupStatus, ZkBackend};
pub use config::{VersionInfo, ZkConfig};
pub use error::ZkError;
pub use ext::{ZkProofExtension, ZK_PROVER_KEY};
pub use prover::ZkProver;
pub use traits::Provable;
pub use witness::{input_map, CompiledCircuit, WitnessGenerator};

// Re-export circuit implementations (they implement Provable)
pub use e3_pvss::circuits::pk_bfv::circuit::PkBfvCircuit;
