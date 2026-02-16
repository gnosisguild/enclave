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
    Sequenced, ShareComputationProofSigned, SignedProofPayload, ThresholdShare,
    ThresholdShareCreated, ThresholdSharePending, TypedEvent, ZkRequest, ZkResponse,
};
use e3_utils::NotifySync;
use tracing::{error, info};

#[derive(Clone, Debug)]
enum ThresholdProofKind {
    PkGeneration,
    SkShareComputation,
    ESmShareComputation,
}

#[derive(Clone, Debug)]
struct PendingProofRequest {
    e3_id: E3id,
    key: Arc<EncryptionKey>,
}

#[derive(Clone, Debug)]
struct PendingThresholdProofs {
    e3_id: E3id,
    full_share: Arc<ThresholdShare>,
    ec: EventContext<Sequenced>,
    pk_generation_proof: Option<Proof>,
    sk_share_computation_proof: Option<Proof>,
    e_sm_share_computation_proof: Option<Proof>,
}

impl PendingThresholdProofs {
    fn new(e3_id: E3id, full_share: Arc<ThresholdShare>, ec: EventContext<Sequenced>) -> Self {
        Self {
            e3_id,
            full_share,
            ec,
            pk_generation_proof: None,
            sk_share_computation_proof: None,
            e_sm_share_computation_proof: None,
        }
    }

    fn is_complete(&self) -> bool {
        self.pk_generation_proof.is_some()
            && self.sk_share_computation_proof.is_some()
            && self.e_sm_share_computation_proof.is_some()
    }

    fn store_proof(&mut self, kind: &ThresholdProofKind, proof: Proof) {
        match kind {
            ThresholdProofKind::PkGeneration => self.pk_generation_proof = Some(proof),
            ThresholdProofKind::SkShareComputation => self.sk_share_computation_proof = Some(proof),
            ThresholdProofKind::ESmShareComputation => {
                self.e_sm_share_computation_proof = Some(proof)
            }
        }
    }
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
    threshold_correlation: HashMap<CorrelationId, (E3id, ThresholdProofKind)>,
    pending_threshold: HashMap<E3id, PendingThresholdProofs>,
}

impl ProofRequestActor {
    pub fn new(bus: &BusHandle, signer: PrivateKeySigner) -> Self {
        Self {
            bus: bus.clone(),
            signer,
            pending: HashMap::new(),
            pending_threshold: HashMap::new(),
            threshold_correlation: HashMap::new(),
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
        let e3_id = msg.e3_id.clone();

        self.pending_threshold.insert(
            e3_id.clone(),
            PendingThresholdProofs::new(e3_id.clone(), msg.full_share.clone(), ec.clone()),
        );

        // C1: PkGeneration
        let t1_corr = CorrelationId::new();
        self.threshold_correlation
            .insert(t1_corr, (e3_id.clone(), ThresholdProofKind::PkGeneration));
        info!("Requesting T1 PkGeneration proof");
        if let Err(err) = self.bus.publish(
            ComputeRequest::zk(
                ZkRequest::PkGeneration(msg.proof_request),
                t1_corr,
                e3_id.clone(),
            ),
            ec.clone(),
        ) {
            error!("Failed to publish T1 proof request: {err}");
            self.threshold_correlation.remove(&t1_corr);
            self.pending_threshold.remove(&e3_id);
            return;
        }

        // C2a: SkShareComputation
        let t2a_corr = CorrelationId::new();
        self.threshold_correlation.insert(
            t2a_corr,
            (e3_id.clone(), ThresholdProofKind::SkShareComputation),
        );
        info!("Requesting T2a SkShareComputation proof");
        if let Err(err) = self.bus.publish(
            ComputeRequest::zk(
                ZkRequest::ShareComputation(msg.sk_share_computation_request),
                t2a_corr,
                e3_id.clone(),
            ),
            ec.clone(),
        ) {
            error!("Failed to publish T2a proof request: {err}");
            self.threshold_correlation
                .retain(|_, (eid, _)| *eid != e3_id);
            self.pending_threshold.remove(&e3_id);
            return;
        }

        // C2b: ESmShareComputation
        let t2b_corr = CorrelationId::new();
        self.threshold_correlation.insert(
            t2b_corr,
            (e3_id.clone(), ThresholdProofKind::ESmShareComputation),
        );
        info!("Requesting T2b ESmShareComputation proof");
        if let Err(err) = self.bus.publish(
            ComputeRequest::zk(
                ZkRequest::ShareComputation(msg.e_sm_share_computation_request),
                t2b_corr,
                e3_id.clone(),
            ),
            ec.clone(),
        ) {
            error!("Failed to publish T2b proof request: {err}");
            self.threshold_correlation
                .retain(|_, (eid, _)| *eid != e3_id);
            self.pending_threshold.remove(&e3_id);
        }
    }

    fn handle_compute_response(&mut self, msg: TypedEvent<ComputeResponse>) {
        let (msg, ec) = msg.into_components();
        match &msg.response {
            ComputeResponseKind::Zk(ZkResponse::PkBfv(resp)) => {
                self.handle_pk_bfv_response(&msg.correlation_id, resp.proof.clone(), &ec);
            }
            ComputeResponseKind::Zk(ZkResponse::PkGeneration(resp)) => {
                self.handle_threshold_proof_response(&msg.correlation_id, resp.proof.clone());
            }
            ComputeResponseKind::Zk(ZkResponse::ShareComputation(resp)) => {
                self.handle_threshold_proof_response(&msg.correlation_id, resp.proof.clone());
            }
            _ => {}
        }
    }

    fn handle_threshold_proof_response(&mut self, correlation_id: &CorrelationId, proof: Proof) {
        let Some((e3_id, kind)) = self.threshold_correlation.remove(correlation_id) else {
            return;
        };

        let Some(pending) = self.pending_threshold.get_mut(&e3_id) else {
            error!(
                "No pending threshold proofs for E3 {} — orphan correlation",
                e3_id
            );
            return;
        };

        info!("Received {:?} proof for E3 {}", kind, e3_id);
        pending.store_proof(&kind, proof);

        if pending.is_complete() {
            info!("All 3 threshold proofs complete for E3 {}", e3_id);
            let pending = self.pending_threshold.remove(&e3_id).unwrap();
            self.publish_threshold_share_with_proofs(pending);
        }
    }

    fn sign_proof(
        &self,
        e3_id: &E3id,
        proof_type: ProofType,
        proof: Proof,
    ) -> Option<SignedProofPayload> {
        let payload = ProofPayload {
            e3_id: e3_id.clone(),
            proof_type,
            proof,
        };
        match SignedProofPayload::sign(payload, &self.signer) {
            Ok(signed) => Some(signed),
            Err(err) => {
                error!("Failed to sign {:?} proof: {err}", proof_type);
                None
            }
        }
    }

    fn publish_threshold_share_with_proofs(&mut self, pending: PendingThresholdProofs) {
        let e3_id = &pending.e3_id;
        let party_id = pending.full_share.party_id;
        let ec = &pending.ec;

        let Some(signed_pk_gen) = self.sign_proof(
            e3_id,
            ProofType::T1PkGeneration,
            pending.pk_generation_proof.expect("checked"),
        ) else {
            error!("Failed to sign T1 proof — shares will not be published");
            return;
        };

        let Some(signed_sk_share) = self.sign_proof(
            e3_id,
            ProofType::T1SkShareComputation,
            pending.sk_share_computation_proof.expect("checked"),
        ) else {
            error!("Failed to sign T2a proof — shares will not be published");
            return;
        };

        let Some(signed_e_sm_share) = self.sign_proof(
            e3_id,
            ProofType::T1ESmShareComputation,
            pending.e_sm_share_computation_proof.expect("checked"),
        ) else {
            error!("Failed to sign T2b proof — shares will not be published");
            return;
        };

        info!(
            "All proofs signed for E3 {} party {} (signer: {})",
            e3_id,
            party_id,
            self.signer.address()
        );

        let share = &pending.full_share;
        let num_parties = share.num_parties();

        if let Err(err) = self.bus.publish(
            PkGenerationProofSigned {
                e3_id: e3_id.clone(),
                party_id,
                signed_proof: signed_pk_gen,
            },
            ec.clone(),
        ) {
            error!("Failed to publish PkGenerationProofSigned: {err}");
        }

        if let Err(err) = self.bus.publish(
            ShareComputationProofSigned {
                e3_id: e3_id.clone(),
                party_id,
                proof_type: ProofType::T1SkShareComputation,
                signed_proof: signed_sk_share,
            },
            ec.clone(),
        ) {
            error!("Failed to publish SkShareComputationProofSigned: {err}");
        }

        if let Err(err) = self.bus.publish(
            ShareComputationProofSigned {
                e3_id: e3_id.clone(),
                party_id,
                proof_type: ProofType::T1ESmShareComputation,
                signed_proof: signed_e_sm_share,
            },
            ec.clone(),
        ) {
            error!("Failed to publish ESmShareComputationProofSigned: {err}");
        }

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

        if let Some((e3_id, kind)) = self.threshold_correlation.remove(msg.correlation_id()) {
            error!(
                "T1 {:?} proof request failed for E3 {}: {err} — threshold share will not be published without proof",
                kind, e3_id
            );
            self.threshold_correlation
                .retain(|_, (eid, _)| *eid != e3_id);
            self.pending_threshold.remove(&e3_id);
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
