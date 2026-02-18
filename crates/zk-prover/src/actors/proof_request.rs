// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::collections::HashMap;
use std::sync::Arc;

use actix::{Actor, Addr, Context, Handler};
use alloy::signers::local::PrivateKeySigner;
use e3_events::{
    BusHandle, ComputeRequest, ComputeRequestError, ComputeRequestErrorKind, ComputeResponse,
    ComputeResponseKind, CorrelationId, E3id, EnclaveEvent, EnclaveEventData, EncryptionKey,
    EncryptionKeyCreated, EncryptionKeyPending, EventContext, EventPublisher, EventSubscriber,
    EventType, PkBfvProofRequest, PkGenerationProofSigned, Proof, ProofPayload, ProofType,
    Sequenced, SignedProofPayload, ThresholdShare, ThresholdShareCreated, ThresholdSharePending,
    TypedEvent, ZkRequest, ZkResponse,
};
use e3_utils::NotifySync;
use tracing::{error, info};

#[derive(Clone, Debug)]
struct PendingProofRequest {
    e3_id: E3id,
    key: Arc<EncryptionKey>,
}

#[derive(Clone, Debug)]
struct PendingThresholdShareProof {
    e3_id: E3id,
    full_share: Arc<ThresholdShare>,
}

/// Core actor that handles encryption key proof requests.
///
/// Proofs are always wrapped in a [`SignedProofPayload`] before being published,
/// enabling fault attribution via the signed proof model.
/// A signer is required — if signing fails, the proof is not published.
pub struct ProofRequestActor {
    bus: BusHandle,
    signer: PrivateKeySigner,
    pending: HashMap<CorrelationId, PendingProofRequest>,
    pending_threshold: HashMap<CorrelationId, PendingThresholdShareProof>,
}

impl ProofRequestActor {
    pub fn new(bus: &BusHandle, signer: PrivateKeySigner) -> Self {
        Self {
            bus: bus.clone(),
            signer,
            pending: HashMap::new(),
            pending_threshold: HashMap::new(),
        }
    }

    pub fn setup(bus: &BusHandle, signer: PrivateKeySigner) -> Addr<Self> {
        let addr = Self::new(bus, signer).start();
        bus.subscribe(EventType::EncryptionKeyPending, addr.clone().into());
        bus.subscribe(EventType::ComputeResponse, addr.clone().into());
        bus.subscribe(EventType::ComputeRequestError, addr.clone().into());
        bus.subscribe(EventType::ThresholdSharePending, addr.clone().into());
        addr
    }

    fn handle_encryption_key_pending(&mut self, msg: TypedEvent<EncryptionKeyPending>) {
        let (msg, ec) = msg.into_components();
        let correlation_id = CorrelationId::new();
        self.pending.insert(
            correlation_id,
            PendingProofRequest {
                e3_id: msg.e3_id.clone(),
                key: msg.key.clone(),
            },
        );

        let request = ComputeRequest::zk(
            ZkRequest::PkBfv(PkBfvProofRequest::new(
                msg.key.pk_bfv.clone(),
                msg.params_preset,
            )),
            correlation_id,
            msg.e3_id,
        );

        info!("Requesting T0 proof generation");
        if let Err(err) = self.bus.publish(request, ec) {
            error!("Failed to publish ZK proof request: {err}");
            self.pending.remove(&correlation_id);
        }
    }

    fn handle_threshold_share_pending(&mut self, msg: TypedEvent<ThresholdSharePending>) {
        let (msg, ec) = msg.into_components();
        let correlation_id = CorrelationId::new();
        self.pending_threshold.insert(
            correlation_id,
            PendingThresholdShareProof {
                e3_id: msg.e3_id.clone(),
                full_share: msg.full_share.clone(),
            },
        );

        let request = ComputeRequest::zk(
            ZkRequest::PkGeneration(msg.proof_request),
            correlation_id,
            msg.e3_id,
        );

        info!("Requesting T1 PkGeneration proof generation");
        if let Err(err) = self.bus.publish(request, ec) {
            error!("Failed to publish ZK proof request: {err}");
            self.pending_threshold.remove(&correlation_id);
        }
    }

    fn handle_compute_response(&mut self, msg: TypedEvent<ComputeResponse>) {
        let (msg, ec) = msg.into_components();
        match &msg.response {
            ComputeResponseKind::Zk(ZkResponse::PkBfv(resp)) => {
                self.handle_pk_bfv_response(&msg.correlation_id, resp.proof.clone(), &ec);
            }
            ComputeResponseKind::Zk(ZkResponse::PkGeneration(resp)) => {
                self.handle_pk_generation_response(&msg.correlation_id, resp.proof.clone(), &ec);
            }
            _ => {}
        }
    }

    fn handle_pk_generation_response(
        &mut self,
        correlation_id: &CorrelationId,
        proof: Proof,
        ec: &EventContext<Sequenced>,
    ) {
        let Some(pending) = self.pending_threshold.remove(correlation_id) else {
            error!(
                "Received PkBfv ComputeResponse with correlation_id {:?} but no matching pending request found.",
                correlation_id
            );
            return;
        };

        let payload = ProofPayload {
            e3_id: pending.e3_id.clone(),
            proof_type: ProofType::T1PkGeneration,
            proof: proof.clone(),
        };

        let signed = match SignedProofPayload::sign(payload, &self.signer) {
            Ok(s) => {
                info!(
                    "Signed T1 PkGeneration proof for party {} (signer: {})",
                    pending.full_share.party_id,
                    self.signer.address()
                );
                s
            }
            Err(err) => {
                error!("Failed to sign T1 PkGeneration proof payload: {err} — shares will not be published");
                return;
            }
        };

        let party_id = pending.full_share.party_id;
        let e3_id = pending.e3_id.clone();

        info!(
            "Publishing PkGenerationProofSigned for E3 {} party {}",
            pending.e3_id, party_id
        );
        if let Err(err) = self.bus.publish(
            PkGenerationProofSigned {
                e3_id: pending.e3_id,
                party_id,
                signed_proof: signed,
            },
            ec.clone(),
        ) {
            error!("Failed to publish PkGenerationProofSigned: {err}");
        }

        let share = &pending.full_share;

        // Publish per-party shares
        let num_parties = share.num_parties();
        info!(
            "Publishing ThresholdShareCreated for E3 {} to {} parties",
            e3_id, num_parties
        );

        for recipient_party_id in 0..num_parties {
            if let Some(party_share) = share.extract_for_party(recipient_party_id) {
                if let Err(err) = self.bus.publish(
                    ThresholdShareCreated {
                        e3_id: e3_id.clone(),
                        share: Arc::new(party_share),
                        target_party_id: recipient_party_id as u64,
                        external: false,
                    },
                    ec.clone(),
                ) {
                    error!(
                        "Failed to publish ThresholdShareCreated for party {}: {err}",
                        recipient_party_id
                    );
                }
            } else {
                error!("Failed to extract share for party {}", recipient_party_id);
            }
        }
    }

    fn handle_pk_bfv_response(
        &mut self,
        correlation_id: &CorrelationId,
        proof: Proof,
        ec: &EventContext<Sequenced>,
    ) {
        let Some(pending) = self.pending.remove(&correlation_id) else {
            error!(
                "Received PkBfv ComputeResponse with correlation_id {:?} but no matching pending request found.",
                correlation_id
            );
            return;
        };

        let mut key = (*pending.key).clone();
        key.proof = Some(proof.clone());

        // Always sign the proof payload — unsigned proofs are not published
        let payload = ProofPayload {
            e3_id: pending.e3_id.clone(),
            proof_type: ProofType::T0PkBfv,
            proof: proof.clone(),
        };

        match SignedProofPayload::sign(payload, &self.signer) {
            Ok(signed) => {
                info!(
                    "Signed T0 proof for party {} (signer: {})",
                    key.party_id,
                    self.signer.address()
                );
                key.signed_payload = Some(signed);
            }
            Err(err) => {
                error!("Failed to sign T0 proof payload: {err} — proof will not be published");
                return;
            }
        }

        if let Err(err) = self.bus.publish(
            EncryptionKeyCreated {
                e3_id: pending.e3_id,
                key: Arc::new(key),
                external: false,
            },
            ec.clone(),
        ) {
            error!("Failed to publish EncryptionKeyCreated: {err}");
        }
    }

    fn handle_compute_request_error(&mut self, msg: TypedEvent<ComputeRequestError>) {
        let ComputeRequestErrorKind::Zk(err) = msg.get_err() else {
            return;
        };

        if let Some(pending) = self.pending.remove(msg.correlation_id()) {
            error!(
                "T0 proof request failed for E3 {}: {err} — key will not be published without proof",
                pending.e3_id
            );
        }

        if let Some(pending) = self.pending_threshold.remove(msg.correlation_id()) {
            error!(
                "T1 PkShareGeneration proof request failed for E3 {}: {err} — threshold share will not be published without proof", pending.e3_id
            )
        }
    }
}

impl Actor for ProofRequestActor {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for ProofRequestActor {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();

        match msg {
            EnclaveEventData::EncryptionKeyPending(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ThresholdSharePending(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ComputeResponse(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ComputeRequestError(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            _ => (),
        }
    }
}

impl Handler<TypedEvent<EncryptionKeyPending>> for ProofRequestActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<EncryptionKeyPending>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_encryption_key_pending(msg)
    }
}

impl Handler<TypedEvent<ThresholdSharePending>> for ProofRequestActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ThresholdSharePending>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_threshold_share_pending(msg);
    }
}

impl Handler<TypedEvent<ComputeResponse>> for ProofRequestActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ComputeResponse>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_compute_response(msg)
    }
}

impl Handler<TypedEvent<ComputeRequestError>> for ProofRequestActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ComputeRequestError>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_compute_request_error(msg)
    }
}
