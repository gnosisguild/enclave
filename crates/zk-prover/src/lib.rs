// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod actor;
mod backend;
mod circuits;
mod config;
mod error;
mod prover;
mod traits;
mod witness;

pub use actor::ZkActor;
pub use backend::{SetupStatus, ZkBackend};
pub use config::{verify_checksum, BbTarget, CircuitInfo, VersionInfo, ZkConfig};
pub use e3_pvss::circuits::pk_bfv::circuit::PkBfvCircuit;
pub use error::ZkError;
pub use prover::ZkProver;
pub use traits::Provable;
pub use witness::{input_map, CompiledCircuit, WitnessGenerator};
