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
    setup_zk_actors, AccusationManager, AccusationManagerExtension,
    CommitmentConsistencyCheckerExtension, ProofRequestActor, ProofVerificationActor,
    ShareVerificationActor, ZkActors, ZkVerificationRequest, ZkVerificationResponse,
};

pub use backend::{SetupStatus, ZkBackend};
pub use circuits::aggregation::c3_accumulator::generate_sequential_c3_fold;
pub use circuits::aggregation::c6_accumulator::generate_sequential_c6_fold;
pub use circuits::aggregation::node_dkg_fold::{
    prove_decryption_aggregation_jobs, prove_dkg_aggregation, prove_node_dkg_fold,
    DecryptionAggregationJob, DkgAggregationInput, NodeDkgFoldInput,
};
pub use circuits::aggregation::nodes_fold_accumulator::generate_sequential_nodes_fold;
pub use config::{verify_checksum, BbTarget, CircuitInfo, VersionInfo, ZkConfig};
pub use e3_events::CircuitVariant;
pub use e3_zk_helpers::circuits::dkg::pk::circuit::PkCircuit;
pub use error::ZkError;
pub use prover::ZkProver;
pub use traits::Provable;
pub use witness::{input_map, CompiledCircuit, WitnessGenerator};
