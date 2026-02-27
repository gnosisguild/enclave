// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Verifies `EncryptionKeyReceived` events: recovers ECDSA address, delegates
//! ZK proof to `ZkActor`, and on failure emits [`SignedProofFailed`] for
//! on-chain fault attribution.

use std::collections::HashMap;
use std::sync::Arc;

use actix::{Actor, Addr, AsyncContext, Context, Handler, Message, Recipient};
use alloy::primitives::Address;
use e3_events::{
    BusHandle, E3id, EnclaveEvent, EnclaveEventData, EncryptionKey, EncryptionKeyCreated,
    EncryptionKeyReceived, EventContext, EventPublisher, EventSubscriber, EventType, Proof,
    Sequenced, SignedProofFailed, SignedProofPayload, TypedEvent,
};
use e3_utils::NotifySync;
use tracing::{error, info, warn};

#[derive(Debug, Message)]
#[rtype(result = "()")]
pub struct ZkVerificationRequest {
    pub proof: Proof,
    pub e3_id: E3id,
    pub key: Arc<EncryptionKey>,
    pub sender: Recipient<TypedEvent<ZkVerificationResponse>>,
}

#[derive(Debug, Clone, Message)]
#[rtype(result = "()")]
pub struct ZkVerificationResponse {
    pub verified: bool,
    pub error: Option<String>,
    pub e3_id: E3id,
    pub key: Arc<EncryptionKey>,
}

#[derive(Clone, Debug)]
struct PendingVerification {
    signed_payload: SignedProofPayload,
    recovered_signer: Address,
}

pub struct ProofVerificationActor {
    bus: BusHandle,
    verifier: Recipient<TypedEvent<ZkVerificationRequest>>,
    pending: HashMap<(E3id, u64), PendingVerification>,
}

impl ProofVerificationActor {
    pub fn new(bus: &BusHandle, verifier: Recipient<TypedEvent<ZkVerificationRequest>>) -> Self {
        Self {
            bus: bus.clone(),
            verifier,
            pending: HashMap::new(),
        }
    }

    pub fn setup(
        bus: &BusHandle,
        verifier: Recipient<TypedEvent<ZkVerificationRequest>>,
    ) -> Addr<Self> {
        let addr = Self::new(bus, verifier).start();
        bus.subscribe(EventType::EncryptionKeyReceived, addr.clone().into());
        addr
    }

    fn handle_encryption_key_received(
        &mut self,
        msg: TypedEvent<EncryptionKeyReceived>,
        ctx: &Context<Self>,
    ) {
        let (msg, ec) = msg.into_components();
        let Some(ref proof) = msg.key.proof else {
            error!(
                "External key from party {} is missing T0 proof - rejecting",
                msg.key.party_id
            );
            return;
        };

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

        self.pending.insert(
            (msg.e3_id.clone(), msg.key.party_id),
            PendingVerification {
                signed_payload: signed,
                recovered_signer: recovered_address,
            },
        );

        let request = TypedEvent::new(
            ZkVerificationRequest {
                proof: proof.clone(),
                e3_id: msg.e3_id,
                key: msg.key,
                sender: ctx.address().recipient(),
            },
            ec,
        );

        self.verifier.do_send(request);
    }

    fn publish_key_created(
        &self,
        e3_id: E3id,
        key: Arc<EncryptionKey>,
        ec: EventContext<Sequenced>,
    ) {
        if let Err(err) = self.bus.publish(
            EncryptionKeyCreated {
                e3_id,
                key,
                external: true,
            },
            ec,
        ) {
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
        let (msg, ec) = msg.into_components();
        match msg {
            EnclaveEventData::EncryptionKeyReceived(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            _ => (),
        }
    }
}

impl Handler<TypedEvent<EncryptionKeyReceived>> for ProofVerificationActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<EncryptionKeyReceived>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_encryption_key_received(msg, ctx)
    }
}

impl Handler<TypedEvent<ZkVerificationResponse>> for ProofVerificationActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ZkVerificationResponse>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let (msg, ec) = msg.into_components();
        let pending_key = (msg.e3_id.clone(), msg.key.party_id);
        let pending = self.pending.remove(&pending_key);

        if msg.verified {
            info!(
                "T0 proof verified for party {} - accepting key",
                msg.key.party_id
            );
            self.publish_key_created(msg.e3_id, msg.key, ec.clone());
        } else {
            let error_msg = msg.error.unwrap_or_else(|| "unknown error".to_string());
            error!(
                "T0 proof verification FAILED for party {} - rejecting key and stopping E3: {}",
                msg.key.party_id, error_msg
            );

            if let Some(PendingVerification {
                signed_payload,
                recovered_signer,
            }) = pending
            {
                warn!(
                    "Emitting SignedProofFailed for party {} (address: {recovered_signer})",
                    msg.key.party_id
                );
                if let Err(err) = self.bus.publish(
                    SignedProofFailed {
                        e3_id: msg.e3_id.clone(),
                        faulting_node: recovered_signer,
                        proof_type: signed_payload.payload.proof_type,
                        signed_payload,
                    },
                    ec.clone(),
                ) {
                    error!("Failed to publish SignedProofFailed: {err}");
                }
            }

            // NOTE: We do NOT emit E3Failed here. The on-chain SlashingManager
            // will expel the faulting node and check if the committee drops below
            // threshold. If it does, the contract emits E3Failed on-chain, which
            // the EVM reader picks up and propagates to all actors. If the committee
            // is still above threshold, the DKG continues with N-1 nodes.
        }
    }
}
