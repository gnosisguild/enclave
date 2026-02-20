// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

mod actors;
mod backend;
mod circuits;
mod config;
mod error;
mod prover;
pub mod test_utils;
mod traits;
mod witness;

pub use actors::{
    setup_zk_actors, ProofRequestActor, ProofVerificationActor, ZkActors, ZkVerificationRequest,
    ZkVerificationResponse,
};

pub use backend::{SetupStatus, ZkBackend};
pub use config::{verify_checksum, BbTarget, CircuitInfo, VersionInfo, ZkConfig};
pub use e3_zk_helpers::circuits::dkg::pk::circuit::PkCircuit;
pub use error::ZkError;
pub use prover::ZkProver;
pub use traits::Provable;
pub use witness::{input_map, CompiledCircuit, WitnessGenerator};
