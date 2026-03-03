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
//! - [`ShareVerificationActor`]: Handles ECDSA + ZK verification for C2/C3/C4 share proofs
//!
//! ### IO Actors (File System Operations)
//! - [`ZkActor`]: Performs actual proof generation/verification using disk-based circuits and bb binary
//!
//! ## Usage
//!
//! ```rust,ignore
//! use e3_zk_prover::{ZkBackend, setup_zk_actors};
//! use e3_events::BusHandle;
//! use alloy::signers::local::PrivateKeySigner;
//!
//! let bus = BusHandle::default();
//! let backend = ZkBackend::with_default_dir().await?;
//! let signer = PrivateKeySigner::random();
//!
//! // Setup all actors with proper separation of concerns
//! setup_zk_actors(&bus, &backend, signer);
//! ```

pub mod proof_request;
pub mod proof_verification;
pub mod share_verification;
pub mod zk_actor;

pub use proof_request::ProofRequestActor;
pub use proof_verification::{
    ProofVerificationActor, ZkVerificationRequest, ZkVerificationResponse,
};
pub use share_verification::ShareVerificationActor;
pub use zk_actor::ZkActor;

use actix::{Actor, Addr};
use alloy::signers::local::PrivateKeySigner;
use e3_events::BusHandle;

use crate::ZkBackend;

/// Setup all ZK-related actors with proper separation of concerns.
///
/// Requires a `ZkBackend` for proof generation/verification and a
/// `PrivateKeySigner` for signing proofs (fault attribution).
pub fn setup_zk_actors(bus: &BusHandle, backend: &ZkBackend, signer: PrivateKeySigner) -> ZkActors {
    let zk_actor = ZkActor::new(backend).start();
    let verifier = zk_actor.clone().recipient();

    let proof_request = ProofRequestActor::setup(bus, signer);
    let proof_verification = ProofVerificationActor::setup(bus, verifier);
    let share_verification = ShareVerificationActor::setup(bus);

    ZkActors {
        zk_actor,
        proof_request,
        proof_verification,
        share_verification,
    }
}

/// Container for all ZK-related actor addresses.
pub struct ZkActors {
    pub zk_actor: Addr<ZkActor>,
    pub proof_request: Addr<ProofRequestActor>,
    pub proof_verification: Addr<ProofVerificationActor>,
    pub share_verification: Addr<ShareVerificationActor>,
}
