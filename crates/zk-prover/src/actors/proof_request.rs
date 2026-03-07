// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;

use actix::{Actor, Addr, Context, Handler};
use alloy::signers::local::PrivateKeySigner;
use e3_events::{
    AggregationProofPending, AggregationProofSigned, BusHandle, ComputeRequest,
    ComputeRequestError, ComputeRequestErrorKind, ComputeResponse, ComputeResponseKind,
    CorrelationId, DecryptionKeyShared, DecryptionShareProofSigned, DecryptionShareProofsPending,
    DecryptionshareCreated, DkgProofSigned, E3Failed, E3Stage, E3id, EnclaveEvent,
    EnclaveEventData, EncryptionKey, EncryptionKeyCreated, EncryptionKeyPending, EventContext,
    EventPublisher, EventSubscriber, EventType, FailureReason, PkAggregationProofPending,
    PkAggregationProofSigned, PkBfvProofRequest, PkGenerationProofSigned, Proof, ProofPayload,
    ProofType, Sequenced, ShareDecryptionProofPending, SignedProofPayload, ThresholdShare,
    ThresholdShareCreated, ThresholdSharePending, TypedEvent, ZkRequest, ZkResponse,
};
use e3_utils::utility_types::ArcBytes;
use e3_utils::NotifySync;
use tracing::{error, info, warn};

#[derive(Clone, Debug)]
enum ThresholdProofKind {
    PkGeneration,
    SkShareComputation,
    ESmShareComputation,
    SkShareEncryption {
        recipient_party_id: usize,
        row_index: usize,
    },
    ESmShareEncryption {
        esi_index: usize,
        recipient_party_id: usize,
        row_index: usize,
    },
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
    /// C3a proofs: keyed by (recipient_party_id, row_index)
    sk_share_encryption_proofs: HashMap<(usize, usize), Proof>,
    expected_sk_enc_count: usize,
    /// C3b proofs: keyed by (esi_index, recipient_party_id, row_index)
    e_sm_share_encryption_proofs: HashMap<(usize, usize, usize), Proof>,
    expected_e_sm_enc_count: usize,
    /// Maps positional index to real party_id (from ThresholdSharePending).
    recipient_party_ids: Vec<u64>,
}

impl PendingThresholdProofs {
    fn new(
        e3_id: E3id,
        full_share: Arc<ThresholdShare>,
        ec: EventContext<Sequenced>,
        expected_sk_enc_count: usize,
        expected_e_sm_enc_count: usize,
        recipient_party_ids: Vec<u64>,
    ) -> Self {
        Self {
            e3_id,
            full_share,
            ec,
            pk_generation_proof: None,
            sk_share_computation_proof: None,
            e_sm_share_computation_proof: None,
            sk_share_encryption_proofs: HashMap::new(),
            expected_sk_enc_count,
            e_sm_share_encryption_proofs: HashMap::new(),
            expected_e_sm_enc_count,
            recipient_party_ids,
        }
    }

    fn is_complete(&self) -> bool {
        self.pk_generation_proof.is_some()
            && self.sk_share_computation_proof.is_some()
            && self.e_sm_share_computation_proof.is_some()
            && self.sk_share_encryption_proofs.len() == self.expected_sk_enc_count
            && self.e_sm_share_encryption_proofs.len() == self.expected_e_sm_enc_count
    }

    fn store_proof(&mut self, kind: &ThresholdProofKind, proof: Proof) {
        match kind {
            ThresholdProofKind::PkGeneration => self.pk_generation_proof = Some(proof),
            ThresholdProofKind::SkShareComputation => self.sk_share_computation_proof = Some(proof),
            ThresholdProofKind::ESmShareComputation => {
                self.e_sm_share_computation_proof = Some(proof)
            }
            ThresholdProofKind::SkShareEncryption {
                recipient_party_id,
                row_index,
            } => {
                self.sk_share_encryption_proofs
                    .insert((*recipient_party_id, *row_index), proof);
            }
            ThresholdProofKind::ESmShareEncryption {
                esi_index,
                recipient_party_id,
                row_index,
            } => {
                self.e_sm_share_encryption_proofs
                    .insert((*esi_index, *recipient_party_id, *row_index), proof);
            }
        }
    }

    fn total_expected(&self) -> usize {
        3 + self.expected_sk_enc_count + self.expected_e_sm_enc_count
    }

    fn total_received(&self) -> usize {
        let base = [
            self.pk_generation_proof.is_some(),
            self.sk_share_computation_proof.is_some(),
            self.e_sm_share_computation_proof.is_some(),
        ]
        .iter()
        .filter(|&&v| v)
        .count();
        base + self.sk_share_encryption_proofs.len() + self.e_sm_share_encryption_proofs.len()
    }
}

#[derive(Clone, Debug)]
enum DecryptionProofKind {
    SecretKey,
    SmudgingNoise { esi_idx: usize },
}

/// Pending C4 (DkgShareDecryption) proof generation state.
#[derive(Clone, Debug)]
struct PendingDecryptionProofs {
    party_id: u64,
    node: String,
    sk_poly_sum: ArcBytes,
    es_poly_sum: Vec<ArcBytes>,
    ec: EventContext<Sequenced>,
    sk_proof: Option<Proof>,
    esm_proofs: HashMap<usize, Proof>,
    expected_esm_count: usize,
}

impl PendingDecryptionProofs {
    fn is_complete(&self) -> bool {
        self.sk_proof.is_some()
            && self.esm_proofs.len() == self.expected_esm_count
            && (0..self.expected_esm_count).all(|i| self.esm_proofs.contains_key(&i))
    }
}

/// Pending C5 (PkAggregation) proof generation state.
#[derive(Clone, Debug)]
struct PendingPkAggregationProof {
    ec: EventContext<Sequenced>,
}

/// Pending C6 (ShareDecryptionProof) proof generation state.
#[derive(Clone, Debug)]
struct PendingShareDecryptionProof {
    party_id: u64,
    node: String,
    decryption_share: Vec<ArcBytes>,
    ec: EventContext<Sequenced>,
}

/// Pending C7 (DecryptedSharesAggregation) proof generation state.
#[derive(Clone, Debug)]
struct PendingAggregationProof {
    ec: EventContext<Sequenced>,
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
    /// C4 proof staging: correlation -> (e3_id, kind)
    decryption_correlation: HashMap<CorrelationId, (E3id, DecryptionProofKind)>,
    /// C4 pending proofs per E3
    pending_decryption: HashMap<E3id, PendingDecryptionProofs>,
    /// C6 proof staging: correlation -> e3_id
    share_decryption_correlation: HashMap<CorrelationId, E3id>,
    /// C6 pending proofs per E3
    pending_share_decryption: HashMap<E3id, PendingShareDecryptionProof>,
    /// C5 proof staging: correlation -> e3_id
    pk_aggregation_correlation: HashMap<CorrelationId, E3id>,
    /// C5 pending proofs per E3
    pending_pk_aggregation: HashMap<E3id, PendingPkAggregationProof>,
    /// C7 proof staging: correlation -> e3_id
    aggregation_correlation: HashMap<CorrelationId, E3id>,
    /// C7 pending proofs per E3
    pending_aggregation: HashMap<E3id, PendingAggregationProof>,
}

impl ProofRequestActor {
    pub fn new(bus: &BusHandle, signer: PrivateKeySigner) -> Self {
        Self {
            bus: bus.clone(),
            signer,
            pending: HashMap::new(),
            pending_threshold: HashMap::new(),
            threshold_correlation: HashMap::new(),
            decryption_correlation: HashMap::new(),
            pending_decryption: HashMap::new(),
            share_decryption_correlation: HashMap::new(),
            pending_share_decryption: HashMap::new(),
            pk_aggregation_correlation: HashMap::new(),
            pending_pk_aggregation: HashMap::new(),
            aggregation_correlation: HashMap::new(),
            pending_aggregation: HashMap::new(),
        }
    }

    pub fn setup(bus: &BusHandle, signer: PrivateKeySigner) -> Addr<Self> {
        let addr = Self::new(bus, signer).start();
        bus.subscribe(EventType::EncryptionKeyPending, addr.clone().into());
        bus.subscribe(EventType::ComputeResponse, addr.clone().into());
        bus.subscribe(EventType::ComputeRequestError, addr.clone().into());
        bus.subscribe(EventType::ThresholdSharePending, addr.clone().into());
        bus.subscribe(EventType::DecryptionShareProofsPending, addr.clone().into());
        bus.subscribe(EventType::ShareDecryptionProofPending, addr.clone().into());
        bus.subscribe(EventType::PkAggregationProofPending, addr.clone().into());
        bus.subscribe(EventType::AggregationProofPending, addr.clone().into());
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

        info!("Requesting C0 proof generation");
        if let Err(err) = self.bus.publish(request, ec) {
            error!("Failed to publish ZK proof request: {err}");
            self.pending.remove(&correlation_id);
        }
    }

    fn handle_threshold_share_pending(&mut self, msg: TypedEvent<ThresholdSharePending>) {
        let (msg, ec) = msg.into_components();
        let e3_id = msg.e3_id.clone();

        let sk_enc_count = msg.sk_share_encryption_requests.len();
        let e_sm_enc_count = msg.e_sm_share_encryption_requests.len();

        self.pending_threshold.insert(
            e3_id.clone(),
            PendingThresholdProofs::new(
                e3_id.clone(),
                msg.full_share.clone(),
                ec.clone(),
                sk_enc_count,
                e_sm_enc_count,
                msg.recipient_party_ids,
            ),
        );

        // C1: PkGeneration
        let t1_corr = CorrelationId::new();
        self.threshold_correlation
            .insert(t1_corr, (e3_id.clone(), ThresholdProofKind::PkGeneration));
        info!("Requesting C1 PkGeneration proof");
        if let Err(err) = self.bus.publish(
            ComputeRequest::zk(
                ZkRequest::PkGeneration(msg.proof_request),
                t1_corr,
                e3_id.clone(),
            ),
            ec.clone(),
        ) {
            error!("Failed to publish C1 proof request: {err}");
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
        info!("Requesting C2a SkShareComputation proof");
        if let Err(err) = self.bus.publish(
            ComputeRequest::zk(
                ZkRequest::ShareComputation(msg.sk_share_computation_request),
                t2a_corr,
                e3_id.clone(),
            ),
            ec.clone(),
        ) {
            error!("Failed to publish C2a proof request: {err}");
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
        info!("Requesting C2b ESmShareComputation proof");
        if let Err(err) = self.bus.publish(
            ComputeRequest::zk(
                ZkRequest::ShareComputation(msg.e_sm_share_computation_request),
                t2b_corr,
                e3_id.clone(),
            ),
            ec.clone(),
        ) {
            error!("Failed to publish C2b proof request: {err}");
            self.threshold_correlation
                .retain(|_, (eid, _)| *eid != e3_id);
            self.pending_threshold.remove(&e3_id);
            return;
        }

        // C3a: SkShareEncryption proofs
        info!(
            "Requesting {} C3a SkShareEncryption proofs for E3 {}",
            sk_enc_count, e3_id
        );
        for req in msg.sk_share_encryption_requests {
            let corr = CorrelationId::new();
            self.threshold_correlation.insert(
                corr,
                (
                    e3_id.clone(),
                    ThresholdProofKind::SkShareEncryption {
                        recipient_party_id: req.recipient_party_id,
                        row_index: req.row_index,
                    },
                ),
            );
            if let Err(err) = self.bus.publish(
                ComputeRequest::zk(ZkRequest::ShareEncryption(req), corr, e3_id.clone()),
                ec.clone(),
            ) {
                error!("Failed to publish C3a proof request: {err}");
                self.threshold_correlation
                    .retain(|_, (eid, _)| *eid != e3_id);
                self.pending_threshold.remove(&e3_id);
                return;
            }
        }

        // C3b: ESmShareEncryption proofs
        info!(
            "Requesting {} C3b ESmShareEncryption proofs for E3 {}",
            e_sm_enc_count, e3_id
        );
        for req in msg.e_sm_share_encryption_requests {
            let corr = CorrelationId::new();
            self.threshold_correlation.insert(
                corr,
                (
                    e3_id.clone(),
                    ThresholdProofKind::ESmShareEncryption {
                        esi_index: req.esi_index,
                        recipient_party_id: req.recipient_party_id,
                        row_index: req.row_index,
                    },
                ),
            );
            if let Err(err) = self.bus.publish(
                ComputeRequest::zk(ZkRequest::ShareEncryption(req), corr, e3_id.clone()),
                ec.clone(),
            ) {
                error!("Failed to publish C3b proof request: {err}");
                self.threshold_correlation
                    .retain(|_, (eid, _)| *eid != e3_id);
                self.pending_threshold.remove(&e3_id);
                return;
            }
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
            ComputeResponseKind::Zk(ZkResponse::ShareEncryption(resp)) => {
                self.handle_threshold_proof_response(&msg.correlation_id, resp.proof.clone());
            }
            ComputeResponseKind::Zk(ZkResponse::DkgShareDecryption(resp)) => {
                // Try C4 decryption proof first, then fall back to C1/C2/C3 threshold
                if self
                    .decryption_correlation
                    .contains_key(&msg.correlation_id)
                {
                    self.handle_decryption_proof_response(&msg.correlation_id, resp.proof.clone());
                } else {
                    self.handle_threshold_proof_response(&msg.correlation_id, resp.proof.clone());
                }
            }
            ComputeResponseKind::Zk(ZkResponse::ThresholdShareDecryption(resp)) => {
                self.handle_share_decryption_proof_response(
                    &msg.correlation_id,
                    resp.proofs.clone(),
                );
            }
            ComputeResponseKind::Zk(ZkResponse::PkAggregation(resp)) => {
                self.handle_pk_aggregation_proof_response(&msg.correlation_id, resp.proof.clone());
            }
            ComputeResponseKind::Zk(ZkResponse::DecryptedSharesAggregation(resp)) => {
                self.handle_aggregation_proof_response(&msg.correlation_id, resp.proofs.clone());
            }
            _ => {}
        }
    }

    /// Handle DecryptionShareProofsPending: dispatch C4 proof generation.
    fn handle_decryption_share_proofs_pending(
        &mut self,
        msg: TypedEvent<DecryptionShareProofsPending>,
    ) {
        let (msg, ec) = msg.into_components();
        let e3_id = msg.e3_id.clone();
        let esm_count = msg.esm_requests.len();

        if self.pending_decryption.contains_key(&e3_id) {
            warn!(
                "Duplicate DecryptionShareProofsPending for E3 {} — ignoring",
                e3_id
            );
            return;
        }

        self.pending_decryption.insert(
            e3_id.clone(),
            PendingDecryptionProofs {
                party_id: msg.party_id,
                node: msg.node,
                sk_poly_sum: msg.sk_poly_sum,
                es_poly_sum: msg.es_poly_sum,
                ec: ec.clone(),
                sk_proof: None,
                esm_proofs: HashMap::new(),
                expected_esm_count: esm_count,
            },
        );

        // C4a: SecretKey decryption proof
        let sk_corr = CorrelationId::new();
        self.decryption_correlation
            .insert(sk_corr, (e3_id.clone(), DecryptionProofKind::SecretKey));
        info!(
            "Requesting C4a DkgShareDecryption proof (SecretKey) for E3 {}",
            e3_id
        );
        if let Err(err) = self.bus.publish(
            ComputeRequest::zk(
                ZkRequest::DkgShareDecryption(msg.sk_request),
                sk_corr,
                e3_id.clone(),
            ),
            ec.clone(),
        ) {
            error!("Failed to publish C4a proof request: {err}");
            self.decryption_correlation
                .retain(|_, (eid, _)| *eid != e3_id);
            self.pending_decryption.remove(&e3_id);
            return;
        }

        // C4b: SmudgingNoise decryption proofs
        for (esi_idx, esm_req) in msg.esm_requests.into_iter().enumerate() {
            let esm_corr = CorrelationId::new();
            self.decryption_correlation.insert(
                esm_corr,
                (
                    e3_id.clone(),
                    DecryptionProofKind::SmudgingNoise { esi_idx },
                ),
            );
            info!(
                "Requesting C4b DkgShareDecryption proof (SmudgingNoise[{}]) for E3 {}",
                esi_idx, e3_id
            );
            if let Err(err) = self.bus.publish(
                ComputeRequest::zk(
                    ZkRequest::DkgShareDecryption(esm_req),
                    esm_corr,
                    e3_id.clone(),
                ),
                ec.clone(),
            ) {
                error!("Failed to publish C4b proof request: {err}");
                self.decryption_correlation
                    .retain(|_, (eid, _)| *eid != e3_id);
                self.pending_decryption.remove(&e3_id);
                return;
            }
        }
    }

    /// Handle a C4 proof response — store and check completeness.
    fn handle_decryption_proof_response(&mut self, correlation_id: &CorrelationId, proof: Proof) {
        let Some((e3_id, kind)) = self.decryption_correlation.remove(correlation_id) else {
            return;
        };

        let Some(pending) = self.pending_decryption.get_mut(&e3_id) else {
            error!(
                "No pending decryption proofs for E3 {} — orphan correlation",
                e3_id
            );
            return;
        };

        match kind {
            DecryptionProofKind::SecretKey => {
                info!("Received C4a SK decryption proof for E3 {}", e3_id);
                pending.sk_proof = Some(proof);
            }
            DecryptionProofKind::SmudgingNoise { esi_idx } => {
                info!(
                    "Received C4b ESM decryption proof [{}] for E3 {}",
                    esi_idx, e3_id
                );
                pending.esm_proofs.insert(esi_idx, proof);
            }
        }

        if pending.is_complete() {
            info!(
                "All C4 proofs complete for E3 {} — signing and publishing DecryptionKeyShared",
                e3_id
            );
            let pending = self.pending_decryption.remove(&e3_id).unwrap();
            self.sign_and_publish_decryption_key_shared(&e3_id, pending);
        }
    }

    /// Sign all C4 proofs and publish DecryptionKeyShared (Exchange #3).
    fn sign_and_publish_decryption_key_shared(
        &mut self,
        e3_id: &E3id,
        pending: PendingDecryptionProofs,
    ) {
        // Sign C4a (SK decryption proof)
        let Some(signed_sk) = self.sign_proof(
            e3_id,
            ProofType::T2DkgShareDecryption,
            pending.sk_proof.expect("checked in is_complete"),
        ) else {
            error!("Failed to sign C4a SK proof — DecryptionKeyShared will not be published");
            return;
        };

        // Sign C4b (ESM decryption proofs) in esi_idx order
        let mut signed_esms = Vec::with_capacity(pending.expected_esm_count);
        for idx in 0..pending.expected_esm_count {
            let proof = pending
                .esm_proofs
                .get(&idx)
                .expect("checked in is_complete")
                .clone();
            let Some(signed) = self.sign_proof(e3_id, ProofType::T2DkgShareDecryption, proof)
            else {
                error!(
                    "Failed to sign C4b ESM proof [{}] — DecryptionKeyShared will not be published",
                    idx
                );
                return;
            };
            signed_esms.push(signed);
        }

        info!(
            "All C4 proofs signed for E3 {} party {} (signer: {})",
            e3_id,
            pending.party_id,
            self.signer.address()
        );

        if let Err(err) = self.bus.publish(
            DecryptionKeyShared {
                e3_id: e3_id.clone(),
                party_id: pending.party_id,
                node: pending.node,
                sk_poly_sum: pending.sk_poly_sum,
                es_poly_sum: pending.es_poly_sum,
                signed_sk_decryption_proof: signed_sk,
                signed_esm_decryption_proofs: signed_esms,
                external: false,
            },
            pending.ec,
        ) {
            error!("Failed to publish DecryptionKeyShared: {err}");
        }
    }

    /// Handle ShareDecryptionProofPending: dispatch C6 proof generation.
    fn handle_share_decryption_proof_pending(
        &mut self,
        msg: TypedEvent<ShareDecryptionProofPending>,
    ) {
        let (msg, ec) = msg.into_components();
        let e3_id = msg.e3_id.clone();

        if self.pending_share_decryption.contains_key(&e3_id) {
            warn!(
                "Duplicate ShareDecryptionProofPending for E3 {} — ignoring",
                e3_id
            );
            return;
        }

        self.pending_share_decryption.insert(
            e3_id.clone(),
            PendingShareDecryptionProof {
                party_id: msg.party_id,
                node: msg.node,
                decryption_share: msg.decryption_share,
                ec: ec.clone(),
            },
        );

        let correlation_id = CorrelationId::new();
        self.share_decryption_correlation
            .insert(correlation_id, e3_id.clone());

        info!(
            "Requesting C6 ThresholdShareDecryption proof for E3 {}",
            e3_id
        );
        if let Err(err) = self.bus.publish(
            ComputeRequest::zk(
                ZkRequest::ThresholdShareDecryption(msg.proof_request),
                correlation_id,
                e3_id.clone(),
            ),
            ec,
        ) {
            error!("Failed to publish C6 proof request: {err}");
            self.share_decryption_correlation.remove(&correlation_id);
            self.pending_share_decryption.remove(&e3_id);
        }
    }

    /// Handle C6 proof response — sign proofs and publish DecryptionshareCreated.
    fn handle_share_decryption_proof_response(
        &mut self,
        correlation_id: &CorrelationId,
        proofs: Vec<Proof>,
    ) {
        let Some(e3_id) = self.share_decryption_correlation.remove(correlation_id) else {
            return;
        };

        let Some(pending) = self.pending_share_decryption.remove(&e3_id) else {
            error!(
                "No pending share decryption proof for E3 {} — orphan correlation",
                e3_id
            );
            return;
        };

        // Sign each C6 proof
        let mut signed_proofs = Vec::with_capacity(proofs.len());
        for proof in proofs {
            let Some(signed) = self.sign_proof(&e3_id, ProofType::T5ShareDecryption, proof) else {
                error!("Failed to sign C6 proof — DecryptionshareCreated will not be published");
                return;
            };
            signed_proofs.push(signed);
        }

        info!(
            "All C6 proofs signed for E3 {} party {} (signer: {})",
            e3_id,
            pending.party_id,
            self.signer.address()
        );

        let ec = pending.ec;

        match self.bus.publish(
            DecryptionshareCreated {
                party_id: pending.party_id,
                node: pending.node,
                e3_id: e3_id.clone(),
                decryption_share: pending.decryption_share,
                signed_decryption_proofs: signed_proofs,
            },
            ec.clone(),
        ) {
            Ok(_) => {
                if let Err(err) = self.bus.publish(
                    DecryptionShareProofSigned {
                        e3_id: e3_id.clone(),
                    },
                    ec,
                ) {
                    error!("Failed to publish DecryptionShareProofSigned: {err}");
                }
            }
            Err(err) => {
                error!("Failed to publish DecryptionshareCreated: {err}");
            }
        }
    }

    /// Handle PkAggregationProofPending: dispatch C5 proof generation.
    fn handle_pk_aggregation_proof_pending(&mut self, msg: TypedEvent<PkAggregationProofPending>) {
        let (msg, ec) = msg.into_components();
        let e3_id = msg.e3_id.clone();

        if self.pending_pk_aggregation.contains_key(&e3_id) {
            warn!(
                "Duplicate PkAggregationProofPending for E3 {} — ignoring",
                e3_id
            );
            return;
        }

        self.pending_pk_aggregation
            .insert(e3_id.clone(), PendingPkAggregationProof { ec: ec.clone() });

        let correlation_id = CorrelationId::new();
        self.pk_aggregation_correlation
            .insert(correlation_id, e3_id.clone());

        info!("Requesting C5 PkAggregation proof for E3 {}", e3_id);
        if let Err(err) = self.bus.publish(
            ComputeRequest::zk(
                ZkRequest::PkAggregation(msg.proof_request),
                correlation_id,
                e3_id.clone(),
            ),
            ec,
        ) {
            error!("Failed to publish C5 proof request: {err}");
            self.pk_aggregation_correlation.remove(&correlation_id);
            self.pending_pk_aggregation.remove(&e3_id);
        }
    }

    /// Handle C5 proof response — sign proof and publish PkAggregationProofSigned.
    fn handle_pk_aggregation_proof_response(
        &mut self,
        correlation_id: &CorrelationId,
        proof: Proof,
    ) {
        let Some(e3_id) = self.pk_aggregation_correlation.remove(correlation_id) else {
            return;
        };

        let Some(pending) = self.pending_pk_aggregation.remove(&e3_id) else {
            error!(
                "No pending pk aggregation proof for E3 {} — orphan correlation",
                e3_id
            );
            return;
        };

        let Some(signed) = self.sign_proof(&e3_id, ProofType::C5PkAggregation, proof) else {
            error!("Failed to sign C5 proof — PkAggregationProofSigned will not be published");
            return;
        };

        info!(
            "C5 proof signed for E3 {} (signer: {})",
            e3_id,
            self.signer.address()
        );

        if let Err(err) = self.bus.publish(
            PkAggregationProofSigned {
                e3_id: e3_id.clone(),
                signed_proof: signed,
            },
            pending.ec,
        ) {
            error!("Failed to publish PkAggregationProofSigned: {err}");
        }
    }

    /// Handle AggregationProofPending: dispatch C7 proof generation.
    fn handle_aggregation_proof_pending(&mut self, msg: TypedEvent<AggregationProofPending>) {
        let (msg, ec) = msg.into_components();
        let e3_id = msg.e3_id.clone();

        if self.pending_aggregation.contains_key(&e3_id) {
            warn!(
                "Duplicate AggregationProofPending for E3 {} — ignoring",
                e3_id
            );
            return;
        }

        self.pending_aggregation
            .insert(e3_id.clone(), PendingAggregationProof { ec: ec.clone() });

        let correlation_id = CorrelationId::new();
        self.aggregation_correlation
            .insert(correlation_id, e3_id.clone());

        info!(
            "Requesting C7 DecryptedSharesAggregation proof for E3 {}",
            e3_id
        );
        if let Err(err) = self.bus.publish(
            ComputeRequest::zk(
                ZkRequest::DecryptedSharesAggregation(msg.proof_request),
                correlation_id,
                e3_id.clone(),
            ),
            ec,
        ) {
            error!("Failed to publish C7 proof request: {err}");
            self.aggregation_correlation.remove(&correlation_id);
            self.pending_aggregation.remove(&e3_id);
        }
    }

    /// Handle C7 proof response — sign proofs and publish AggregationProofSigned.
    fn handle_aggregation_proof_response(
        &mut self,
        correlation_id: &CorrelationId,
        proofs: Vec<Proof>,
    ) {
        let Some(e3_id) = self.aggregation_correlation.remove(correlation_id) else {
            return;
        };

        let Some(pending) = self.pending_aggregation.remove(&e3_id) else {
            error!(
                "No pending aggregation proof for E3 {} — orphan correlation",
                e3_id
            );
            return;
        };

        // Sign each C7 proof
        let mut signed_proofs = Vec::with_capacity(proofs.len());
        for proof in proofs {
            let Some(signed) =
                self.sign_proof(&e3_id, ProofType::T6DecryptedSharesAggregation, proof)
            else {
                error!("Failed to sign C7 proof — AggregationProofSigned will not be published");
                return;
            };
            signed_proofs.push(signed);
        }

        info!(
            "All C7 proofs signed for E3 {} (signer: {})",
            e3_id,
            self.signer.address()
        );

        if let Err(err) = self.bus.publish(
            AggregationProofSigned {
                e3_id: e3_id.clone(),
                signed_proofs,
            },
            pending.ec,
        ) {
            error!("Failed to publish AggregationProofSigned: {err}");
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

        pending.store_proof(&kind, proof);
        info!(
            "Received {:?} proof for E3 {} ({}/{})",
            kind,
            e3_id,
            pending.total_received(),
            pending.total_expected()
        );

        if pending.is_complete() {
            info!(
                "All {} threshold proofs complete for E3 {}",
                pending.total_expected(),
                e3_id
            );
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

    fn sign_and_group_proofs(
        &self,
        e3_id: &E3id,
        proof_type: ProofType,
        proofs: impl Iterator<Item = (usize, Proof)>,
    ) -> Option<BTreeMap<usize, Vec<SignedProofPayload>>> {
        let mut map: BTreeMap<usize, Vec<SignedProofPayload>> = BTreeMap::new();
        for (recipient, proof) in proofs {
            let signed = self.sign_proof(e3_id, proof_type, proof)?;
            map.entry(recipient).or_default().push(signed);
        }
        Some(map)
    }

    fn publish_threshold_share_with_proofs(&mut self, pending: PendingThresholdProofs) {
        let e3_id = &pending.e3_id;
        let party_id = pending.full_share.party_id;
        let ec = &pending.ec;

        // Sign C1 (PkGeneration)
        let Some(signed_pk_gen) = self.sign_proof(
            e3_id,
            ProofType::C1PkGeneration,
            pending.pk_generation_proof.expect("checked"),
        ) else {
            error!("Failed to sign C1 proof — shares will not be published");
            return;
        };

        // Sign C2a (SkShareComputation)
        let Some(signed_c2a) = self.sign_proof(
            e3_id,
            ProofType::C2aSkShareComputation,
            pending.sk_share_computation_proof.expect("checked"),
        ) else {
            error!("Failed to sign C2a proof — shares will not be published");
            return;
        };

        // Sign C2b (ESmShareComputation)
        let Some(signed_c2b) = self.sign_proof(
            e3_id,
            ProofType::C2bESmShareComputation,
            pending.e_sm_share_computation_proof.expect("checked"),
        ) else {
            error!("Failed to sign C2b proof — shares will not be published");
            return;
        };

        let Some(signed_c3a_map) = self.sign_and_group_proofs(
            e3_id,
            ProofType::C3aSkShareEncryption,
            pending
                .sk_share_encryption_proofs
                .iter()
                .map(|((recipient, _row), proof)| (*recipient, proof.clone())),
        ) else {
            error!("Failed to sign C3a proofs — shares will not be published");
            return;
        };

        let Some(signed_c3b_map) = self.sign_and_group_proofs(
            e3_id,
            ProofType::C3bESmShareEncryption,
            pending
                .e_sm_share_encryption_proofs
                .iter()
                .map(|((_esi, recipient, _row), proof)| (*recipient, proof.clone())),
        ) else {
            error!("Failed to sign C3b proofs — shares will not be published");
            return;
        };

        info!(
            "All proofs signed for E3 {} party {} (signer: {})",
            e3_id,
            party_id,
            self.signer.address()
        );

        // Publish local proof events for the node's own state tracking
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
            DkgProofSigned {
                e3_id: e3_id.clone(),
                party_id,
                signed_proof: signed_c2a.clone(),
            },
            ec.clone(),
        ) {
            error!("Failed to publish SkDkgProofSigned: {err}");
        }

        if let Err(err) = self.bus.publish(
            DkgProofSigned {
                e3_id: e3_id.clone(),
                party_id,
                signed_proof: signed_c2b.clone(),
            },
            ec.clone(),
        ) {
            error!("Failed to publish ESmDkgProofSigned: {err}");
        }

        // Publish C3a signed proofs (reuse already-signed proofs from signed_c3a_map)
        for signed_proofs in signed_c3a_map.values() {
            for signed in signed_proofs {
                if let Err(err) = self.bus.publish(
                    DkgProofSigned {
                        e3_id: e3_id.clone(),
                        party_id,
                        signed_proof: signed.clone(),
                    },
                    ec.clone(),
                ) {
                    error!("Failed to publish SkShareEncryptionProofSigned: {err}");
                }
            }
        }

        // Publish C3b signed proofs (reuse already-signed proofs from signed_c3b_map)
        for signed_proofs in signed_c3b_map.values() {
            for signed in signed_proofs {
                if let Err(err) = self.bus.publish(
                    DkgProofSigned {
                        e3_id: e3_id.clone(),
                        party_id,
                        signed_proof: signed.clone(),
                    },
                    ec.clone(),
                ) {
                    error!("Failed to publish ESmShareEncryptionProofSigned: {err}");
                }
            }
        }

        // Publish ThresholdShareCreated with proofs attached for each recipient
        let share = &pending.full_share;
        let num_parties = share.num_parties();

        info!(
            "Publishing ThresholdShareCreated for E3 {} to {} parties",
            e3_id, num_parties
        );

        for positional_idx in 0..num_parties {
            if let Some(party_share) = share.extract_for_party(positional_idx) {
                let c3a_proofs = signed_c3a_map
                    .get(&positional_idx)
                    .cloned()
                    .unwrap_or_default();
                let c3b_proofs = signed_c3b_map
                    .get(&positional_idx)
                    .cloned()
                    .unwrap_or_default();

                // Use real party_id from the mapping (positional index may differ
                // from party_id when expelled members cause gaps)
                let real_party_id = pending
                    .recipient_party_ids
                    .get(positional_idx)
                    .copied()
                    .unwrap_or(positional_idx as u64);

                if let Err(err) = self.bus.publish(
                    ThresholdShareCreated {
                        e3_id: e3_id.clone(),
                        share: Arc::new(party_share),
                        target_party_id: real_party_id,
                        external: false,
                        signed_c2a_proof: Some(signed_c2a.clone()),
                        signed_c2b_proof: Some(signed_c2b.clone()),
                        signed_c3a_proofs: c3a_proofs,
                        signed_c3b_proofs: c3b_proofs,
                    },
                    ec.clone(),
                ) {
                    error!(
                        "Failed to publish ThresholdShareCreated for party {} (idx {}): {err}",
                        real_party_id, positional_idx
                    );
                }
            } else {
                error!("Failed to extract share for index {}", positional_idx);
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
            proof_type: ProofType::C0PkBfv,
            proof: proof.clone(),
        };

        match SignedProofPayload::sign(payload, &self.signer) {
            Ok(signed) => {
                info!(
                    "Signed C0 proof for party {} (signer: {})",
                    key.party_id,
                    self.signer.address()
                );
                key.signed_payload = Some(signed);
            }
            Err(err) => {
                error!("Failed to sign C0 proof payload: {err} — proof will not be published");
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
        let (msg, ec) = msg.into_components();
        let ComputeRequestErrorKind::Zk(err) = msg.get_err() else {
            return;
        };

        if let Some(pending) = self.pending.remove(msg.correlation_id()) {
            error!(
                "C0 proof request failed for E3 {}: {err} — key will not be published without proof",
                pending.e3_id
            );
            return;
        }

        if let Some((e3_id, kind)) = self.threshold_correlation.remove(msg.correlation_id()) {
            error!(
                "DKG {:?} proof request failed for E3 {}: {err} — threshold share will not be published without proof",
                kind, e3_id
            );
            self.threshold_correlation
                .retain(|_, (eid, _)| *eid != e3_id);
            self.pending_threshold.remove(&e3_id);
            return;
        }

        if let Some((e3_id, kind)) = self.decryption_correlation.remove(msg.correlation_id()) {
            error!(
                "C4 {:?} proof request failed for E3 {}: {err} — DecryptionKeyShared will not be published",
                kind, e3_id
            );
            self.decryption_correlation
                .retain(|_, (eid, _)| *eid != e3_id);
            self.pending_decryption.remove(&e3_id);
            return;
        }

        if let Some(e3_id) = self
            .share_decryption_correlation
            .remove(msg.correlation_id())
        {
            error!(
                "C6 proof request failed for E3 {}: {err} — DecryptionshareCreated will not be published",
                e3_id
            );
            self.pending_share_decryption.remove(&e3_id);
            if let Err(e) = self.bus.publish(
                E3Failed {
                    e3_id,
                    failed_at_stage: E3Stage::CiphertextReady,
                    reason: FailureReason::DecryptionInvalidShares,
                },
                ec.clone(),
            ) {
                error!("Failed to publish E3Failed for C6 error: {e}");
            }
            return;
        }

        if let Some(e3_id) = self.pk_aggregation_correlation.remove(msg.correlation_id()) {
            error!(
                "C5 proof request failed for E3 {}: {err} — PkAggregationProofSigned will not be published",
                e3_id
            );
            self.pending_pk_aggregation.remove(&e3_id);
            if let Err(e) = self.bus.publish(
                E3Failed {
                    e3_id,
                    failed_at_stage: E3Stage::CommitteeFinalized,
                    reason: FailureReason::DKGInvalidShares,
                },
                ec.clone(),
            ) {
                error!("Failed to publish E3Failed for C5 error: {e}");
            }
            return;
        }

        if let Some(e3_id) = self.aggregation_correlation.remove(msg.correlation_id()) {
            error!(
                "C7 proof request failed for E3 {}: {err} — AggregationProofSigned will not be published",
                e3_id
            );
            self.pending_aggregation.remove(&e3_id);
            if let Err(e) = self.bus.publish(
                E3Failed {
                    e3_id,
                    failed_at_stage: E3Stage::CiphertextReady,
                    reason: FailureReason::DecryptionInvalidShares,
                },
                ec,
            ) {
                error!("Failed to publish E3Failed for C7 error: {e}");
            }
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
            EnclaveEventData::DecryptionShareProofsPending(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ShareDecryptionProofPending(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::PkAggregationProofPending(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::AggregationProofPending(data) => {
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

impl Handler<TypedEvent<DecryptionShareProofsPending>> for ProofRequestActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<DecryptionShareProofsPending>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_decryption_share_proofs_pending(msg)
    }
}

impl Handler<TypedEvent<ShareDecryptionProofPending>> for ProofRequestActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ShareDecryptionProofPending>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_share_decryption_proof_pending(msg)
    }
}

impl Handler<TypedEvent<PkAggregationProofPending>> for ProofRequestActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<PkAggregationProofPending>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_pk_aggregation_proof_pending(msg)
    }
}

impl Handler<TypedEvent<AggregationProofPending>> for ProofRequestActor {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<AggregationProofPending>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_aggregation_proof_pending(msg)
    }
}
