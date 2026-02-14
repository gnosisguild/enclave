// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Core business logic actor for verifying received encryption keys.
//!
//! This actor verifies `EncryptionKeyReceived` events and converts them
//! to `EncryptionKeyCreated` events after validation.
//!
//! ## Signature Verification
//!
//! Every received key must carry a [`SignedProofPayload`]. This actor:
//! 1. Recovers the address from the ECDSA signature.
//! 2. Delegates the ZK proof to `ZkActor` for verification.
//! 3. On ZK failure, emits [`SignedProofFailed`] with the full evidence bundle
//!    and [`E3Failed`] to stop the E3 computation.
//!
//! Keys without a signed proof are rejected outright.

use std::collections::HashMap;
use std::sync::Arc;

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message, Recipient};
use alloy::primitives::Address;
use e3_events::{
    BusHandle, E3Failed, E3Stage, E3id, EnclaveEvent, EnclaveEventData, EncryptionKey,
    EncryptionKeyCreated, EncryptionKeyReceived, Event, EventPublisher, EventSubscriber, EventType,
    FailureReason, Proof, SignedProofFailed, SignedProofPayload,
};
use e3_utils::NotifySync;
use tracing::{error, info, warn};

/// Request to verify a ZK proof.
#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct ZkVerificationRequest {
    pub proof: Proof,
    pub e3_id: E3id,
    pub key: Arc<EncryptionKey>,
    pub sender: Recipient<ZkVerificationResponse>,
}

/// Response from ZK proof verification with context.
#[derive(Debug, Clone, Message)]
#[rtype(result = "()")]
pub struct ZkVerificationResponse {
    pub verified: bool,
    pub error: Option<String>,
    pub e3_id: E3id,
    pub key: Arc<EncryptionKey>,
}

/// Tracks a pending verification including the signed payload for fault evidence.
#[derive(Clone, Debug)]
struct PendingVerification {
    signed_payload: SignedProofPayload,
    recovered_signer: Address,
}

/// Core actor that handles encryption key verification.
///
/// Requires every received key to carry a [`SignedProofPayload`].
/// On ZK verification failure, emits both [`SignedProofFailed`] (for fault
/// attribution) and [`E3Failed`] (to stop the E3 computation).
pub struct ProofVerificationActor {
    bus: BusHandle,
    verifier: Recipient<ZkVerificationRequest>,
    /// Tracks signed payloads for keys currently being verified,
    /// keyed by `(e3_id, party_id)`.
    pending: HashMap<(E3id, u64), PendingVerification>,
}

impl ProofVerificationActor {
    pub fn new(bus: &BusHandle, verifier: Recipient<ZkVerificationRequest>) -> Self {
        Self {
            bus: bus.clone(),
            verifier,
            pending: HashMap::new(),
        }
    }

    pub fn setup(bus: &BusHandle, verifier: Recipient<ZkVerificationRequest>) -> Addr<Self> {
        let addr = Self::new(bus, verifier).start();
        bus.subscribe(EventType::EncryptionKeyReceived, addr.clone().into());
        addr
    }

    fn handle_encryption_key_received(&mut self, msg: EncryptionKeyReceived, ctx: &Context<Self>) {
        let Some(ref proof) = msg.key.proof else {
            error!(
                "External key from party {} is missing T0 proof - rejecting",
                msg.key.party_id
            );
            return;
        };

        // Signed proofs are mandatory — reject keys without a signed payload
        let signed = match &msg.key.signed_payload {
            Some(signed) => signed.clone(),
            None => {
                error!(
                    "Key from party {} has no signed payload - rejecting (signed proofs are required)",
                    msg.key.party_id
                );
                return;
            }
        };

        // Recover the address from the signature
        let recovered_address = match signed.recover_address() {
            Ok(addr) => {
                info!(
                    "Recovered address {} for key from party {}",
                    addr, msg.key.party_id
                );
                addr
            }
            Err(err) => {
                error!(
                    "Invalid signature on key from party {} - rejecting: {err}",
                    msg.key.party_id
                );
                return;
            }
        };

        // Store the signed payload so we can reference it in the verification response
        self.pending.insert(
            (msg.e3_id.clone(), msg.key.party_id),
            PendingVerification {
                signed_payload: signed,
                recovered_signer: recovered_address,
            },
        );

        let request = ZkVerificationRequest {
            proof: proof.clone(),
            e3_id: msg.e3_id,
            key: msg.key,
            sender: ctx.address().recipient(),
        };

        self.verifier.do_send(request);
    }

    fn publish_key_created(&self, e3_id: E3id, key: Arc<EncryptionKey>) {
        if let Err(err) = self.bus.publish(EncryptionKeyCreated {
            e3_id,
            key,
            external: true,
        }) {
            error!("Failed to publish EncryptionKeyCreated: {err}");
        }
    }
}

impl Actor for ProofVerificationActor {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for ProofVerificationActor {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg.into_data() {
            EnclaveEventData::EncryptionKeyReceived(data) => self.notify_sync(ctx, data),
            _ => (),
        }
    }
}

impl Handler<EncryptionKeyReceived> for ProofVerificationActor {
    type Result = ();

    fn handle(&mut self, msg: EncryptionKeyReceived, ctx: &mut Self::Context) -> Self::Result {
        self.handle_encryption_key_received(msg, ctx)
    }
}

impl Handler<ZkVerificationResponse> for ProofVerificationActor {
    type Result = ();

    fn handle(&mut self, msg: ZkVerificationResponse, _ctx: &mut Self::Context) -> Self::Result {
        let pending_key = (msg.e3_id.clone(), msg.key.party_id);
        let pending = self.pending.remove(&pending_key);

        if msg.verified {
            info!(
                "T0 proof verified for party {} - accepting key",
                msg.key.party_id
            );
            self.publish_key_created(msg.e3_id, msg.key);
        } else {
            let error_msg = msg.error.unwrap_or_else(|| "unknown error".to_string());
            error!(
                "T0 proof verification FAILED for party {} - rejecting key and stopping E3: {}",
                msg.key.party_id, error_msg
            );

            // Emit SignedProofFailed for fault attribution
            if let Some(PendingVerification {
                signed_payload,
                recovered_signer,
            }) = pending
            {
                warn!(
                    "Emitting SignedProofFailed for party {} (address: {recovered_signer})",
                    msg.key.party_id
                );
                if let Err(err) = self.bus.publish(SignedProofFailed {
                    e3_id: msg.e3_id.clone(),
                    faulting_node: recovered_signer,
                    proof_type: signed_payload.payload.proof_type,
                    signed_payload,
                }) {
                    error!("Failed to publish SignedProofFailed: {err}");
                }
            }

            // Stop the E3 computation — proof verification failure is fatal
            if let Err(err) = self.bus.publish(E3Failed {
                e3_id: msg.e3_id,
                failed_at_stage: E3Stage::CommitteeFinalized,
                reason: FailureReason::VerificationFailed,
            }) {
                error!("Failed to publish E3Failed: {err}");
            }
        }
    }
}
