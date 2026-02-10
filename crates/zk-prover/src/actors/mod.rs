// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Actor-based components for ZK proof generation and verification.
//!
//! ## Architecture
//!
//! This module follows a clean separation between core business logic and IO operations:
//!
//! ### Core Actors (Business Logic - No IO)
//! - [`ProofRequestActor`]: Converts `EncryptionKeyPending` → `ComputeRequest` and handles responses
//! - [`ProofVerificationActor`]: Verifies `EncryptionKeyReceived` and converts to `EncryptionKeyCreated`
//!
//! ### IO Actors (File System Operations)
//! - [`ZkActor`]: Performs actual proof generation/verification using disk-based circuits and bb binary
//!
//! ## Usage
//!
//! ```rust,ignore
//! use e3_zk_prover::{ZkBackend, setup_zk_actors};
//! use e3_events::BusHandle;
//!
//! let bus = BusHandle::default();
//! let backend = ZkBackend::with_default_dir().await?;
//!
//! // Setup all actors with proper separation of concerns
//! setup_zk_actors(&bus, Some(&backend));
//! ```

pub mod proof_request;
pub mod proof_verification;
pub mod zk_actor;

pub use proof_request::ProofRequestActor;
pub use proof_verification::{
    ProofVerificationActor, ZkVerificationRequest, ZkVerificationResponse,
};
pub use zk_actor::ZkActor;

use actix::{Actor, Addr};
use alloy::signers::{k256::ecdsa::SigningKey, local::LocalSigner};
use e3_events::BusHandle;

use crate::ZkBackend;

/// Setup all ZK-related actors with proper separation of concerns.
///
/// When `backend` is provided:
/// - Creates IO actor (ZkActor) for proof generation/verification
/// - Creates core actors that delegate to IO actor
///
/// When `backend` is None:
/// - Creates core actors without verification capabilities
/// - Proofs are disabled, keys are accepted without verification
///
/// When `signer` is provided:
/// - Proof request actor will sign proofs enabling fault attribution
/// - Without a signer, proofs are still generated but unsigned
pub fn setup_zk_actors(
    bus: &BusHandle,
    backend: Option<&ZkBackend>,
    signer: Option<LocalSigner<SigningKey>>,
) -> ZkActors {
    let (zk_actor, verifier) = if let Some(backend) = backend {
        let zk_actor = ZkActor::new(backend).start();
        let verifier = Some(zk_actor.clone().recipient());
        (Some(zk_actor), verifier)
    } else {
        (None, None)
    };

    let proof_request = ProofRequestActor::setup(bus, backend.is_some(), signer);
    let proof_verification = ProofVerificationActor::setup(bus, verifier);

    ZkActors {
        zk_actor,
        proof_request,
        proof_verification,
    }
}

/// Container for all ZK-related actor addresses.
pub struct ZkActors {
    pub zk_actor: Option<Addr<ZkActor>>,
    pub proof_request: Addr<ProofRequestActor>,
    pub proof_verification: Addr<ProofVerificationActor>,
}
