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
//! When the received key carries a [`SignedProofPayload`], this actor:
//! 1. Recovers the signer address from the ECDSA signature.
//! 2. Delegates the ZK proof to `ZkActor` for verification.
//! 3. On ZK failure, emits [`SignedProofFailed`] with the full evidence bundle
//!    so the `FaultSubmitter` can submit a slash proposal on-chain.
//!
//! This is a CORE actor - it delegates IO operations (verification) to ZkActor.

use std::collections::HashMap;
use std::sync::Arc;

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message, Recipient};
use alloy::primitives::Address;
use e3_events::{
    BusHandle, E3id, EnclaveEvent, EnclaveEventData, EncryptionKey, EncryptionKeyCreated,
    EncryptionKeyReceived, Event, EventPublisher, EventSubscriber, EventType, Proof,
    SignedProofFailed, SignedProofPayload,
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
    signed_payload: Option<SignedProofPayload>,
    recovered_signer: Option<Address>,
}

/// Core actor that handles encryption key verification.
///
/// On ZK verification failure, if the key carried a valid [`SignedProofPayload`],
/// emits a [`SignedProofFailed`] event with the signed evidence bundle.
pub struct ProofVerificationActor {
    bus: BusHandle,
    verifier: Option<Recipient<ZkVerificationRequest>>,
    /// Tracks signed payloads for keys currently being verified,
    /// keyed by `(e3_id, party_id)`.
    pending: HashMap<(E3id, u64), PendingVerification>,
}

impl ProofVerificationActor {
    pub fn new(bus: &BusHandle, verifier: Option<Recipient<ZkVerificationRequest>>) -> Self {
        Self {
            bus: bus.clone(),
            verifier,
            pending: HashMap::new(),
        }
    }

    pub fn setup(
        bus: &BusHandle,
        verifier: Option<Recipient<ZkVerificationRequest>>,
    ) -> Addr<Self> {
        let addr = Self::new(bus, verifier).start();
        bus.subscribe(EventType::EncryptionKeyReceived, addr.clone().into());
        addr
    }

    fn handle_encryption_key_received(&mut self, msg: EncryptionKeyReceived, ctx: &Context<Self>) {
        let Some(ref verifier) = self.verifier else {
            warn!(
                "ZK verifier not available - accepting key from party {} without verification",
                msg.key.party_id
            );
            self.publish_key_created(msg.e3_id, msg.key);
            return;
        };

        let Some(ref proof) = msg.key.proof else {
            warn!(
                "External key from party {} is missing T0 proof - rejecting",
                msg.key.party_id
            );
            return;
        };

        // Validate the signed payload if present
        let (signed_payload, recovered_signer) = if let Some(ref signed) = msg.key.signed_payload {
            match signed.recover_signer() {
                Ok(addr) => {
                    info!(
                        "Recovered signer {} for key from party {}",
                        addr, msg.key.party_id
                    );
                    (Some(signed.clone()), Some(addr))
                }
                Err(err) => {
                    warn!(
                        "Invalid signature on key from party {} - proceeding without \
                             fault attribution: {err}",
                        msg.key.party_id
                    );
                    (None, None)
                }
            }
        } else {
            warn!(
                "Key from party {} has no signed payload - \
                     proof verification will proceed but fault attribution unavailable",
                msg.key.party_id
            );
            (None, None)
        };

        // Store the signed payload so we can reference it in the verification response
        self.pending.insert(
            (msg.e3_id.clone(), msg.key.party_id),
            PendingVerification {
                signed_payload,
                recovered_signer,
            },
        );

        let request = ZkVerificationRequest {
            proof: proof.clone(),
            e3_id: msg.e3_id,
            key: msg.key,
            sender: ctx.address().recipient(),
        };

        verifier.do_send(request);
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
            error!(
                "T0 proof verification FAILED for party {} - rejecting key: {}",
                msg.key.party_id,
                msg.error.unwrap_or_else(|| "unknown error".to_string())
            );

            // If we have a signed payload, emit SignedProofFailed for fault attribution
            if let Some(PendingVerification {
                signed_payload: Some(signed),
                recovered_signer: Some(signer),
            }) = pending
            {
                warn!(
                    "Emitting SignedProofFailed for party {} (signer: {signer})",
                    msg.key.party_id
                );
                if let Err(err) = self.bus.publish(SignedProofFailed {
                    e3_id: msg.e3_id,
                    faulting_node: signer,
                    proof_type: signed.payload.proof_type,
                    signed_payload: signed,
                }) {
                    error!("Failed to publish SignedProofFailed: {err}");
                }
            } else {
                warn!(
                    "No signed payload available for party {} - \
                     fault cannot be attributed on-chain",
                    msg.key.party_id
                );
            }
        }
    }
}
