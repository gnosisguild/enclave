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
    setup_zk_actors, AccusationManager, AccusationManagerExtension, ProofRequestActor,
    ProofVerificationActor, ShareVerificationActor, ZkActors, ZkVerificationRequest,
    ZkVerificationResponse,
};

pub use backend::{SetupStatus, ZkBackend};
pub use circuits::dkg::share_computation::{
    generate_chunk_batch_proof, generate_chunk_proof, generate_share_computation_final_proof,
};
pub use circuits::public_signals;
pub use circuits::recursive_aggregation::{generate_fold_proof, generate_wrapper_proof};
pub use config::{verify_checksum, BbTarget, CircuitInfo, VersionInfo, ZkConfig};
pub use e3_events::CircuitVariant;
pub use e3_zk_helpers::circuits::dkg::pk::circuit::PkCircuit;
pub use error::ZkError;
pub use prover::ZkProver;
pub use traits::Provable;
pub use witness::{input_map, CompiledCircuit, WitnessGenerator};
