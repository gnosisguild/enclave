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
    BusHandle, ComputeRequest, ComputeRequestError, ComputeRequestErrorKind, ComputeResponse,
    ComputeResponseKind, CorrelationId, DecryptionKeyShared, DecryptionShareProofsPending,
    DkgProofSigned, E3id, EnclaveEvent, EnclaveEventData, EncryptionKey, EncryptionKeyCreated,
    EncryptionKeyPending, EventContext, EventPublisher, EventSubscriber, EventType,
    PkBfvProofRequest, PkGenerationProofSigned, Proof, ProofPayload, ProofType, Sequenced,
    SignedProofPayload, ThresholdShare, ThresholdShareCreated, ThresholdSharePending, TypedEvent,
    ZkRequest, ZkResponse,
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
}

impl PendingThresholdProofs {
    fn new(
        e3_id: E3id,
        full_share: Arc<ThresholdShare>,
        ec: EventContext<Sequenced>,
        expected_sk_enc_count: usize,
        expected_e_sm_enc_count: usize,
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
    /// C4 proof staging: correlation → (e3_id, kind)
    decryption_correlation: HashMap<CorrelationId, (E3id, DecryptionProofKind)>,
    /// C4 pending proofs per E3
    pending_decryption: HashMap<E3id, PendingDecryptionProofs>,
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
        }
    }

    pub fn setup(bus: &BusHandle, signer: PrivateKeySigner) -> Addr<Self> {
        let addr = Self::new(bus, signer).start();
        bus.subscribe(EventType::EncryptionKeyPending, addr.clone().into());
        bus.subscribe(EventType::ComputeResponse, addr.clone().into());
        bus.subscribe(EventType::ComputeRequestError, addr.clone().into());
        bus.subscribe(EventType::ThresholdSharePending, addr.clone().into());
        bus.subscribe(EventType::DecryptionShareProofsPending, addr.clone().into());
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

        // Sign C3a proofs (SkShareEncryption) — keyed by (recipient, row)
        let mut signed_c3a_map: BTreeMap<usize, Vec<SignedProofPayload>> = BTreeMap::new();
        for ((_recipient, _row), proof) in &pending.sk_share_encryption_proofs {
            if let Some(signed) =
                self.sign_proof(e3_id, ProofType::C3aSkShareEncryption, proof.clone())
            {
                signed_c3a_map.entry(*_recipient).or_default().push(signed);
            } else {
                error!(
                    "Failed to sign C3a proof for recipient {} — shares will not be published",
                    _recipient
                );
                return;
            }
        }

        // Sign C3b proofs (ESmShareEncryption) — keyed by (esi_index, recipient, row)
        let mut signed_c3b_map: BTreeMap<usize, Vec<SignedProofPayload>> = BTreeMap::new();
        for ((_esi, _recipient, _row), proof) in &pending.e_sm_share_encryption_proofs {
            if let Some(signed) =
                self.sign_proof(e3_id, ProofType::C3bESmShareEncryption, proof.clone())
            {
                signed_c3b_map.entry(*_recipient).or_default().push(signed);
            } else {
                error!(
                    "Failed to sign C3b proof for recipient {} — shares will not be published",
                    _recipient
                );
                return;
            }
        }

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

        for recipient_party_id in 0..num_parties {
            if let Some(party_share) = share.extract_for_party(recipient_party_id) {
                let c3a_proofs = signed_c3a_map
                    .get(&recipient_party_id)
                    .cloned()
                    .unwrap_or_default();
                let c3b_proofs = signed_c3b_map
                    .get(&recipient_party_id)
                    .cloned()
                    .unwrap_or_default();

                if let Err(err) = self.bus.publish(
                    ThresholdShareCreated {
                        e3_id: e3_id.clone(),
                        share: Arc::new(party_share),
                        target_party_id: recipient_party_id as u64,
                        external: false,
                        signed_c2a_proof: Some(signed_c2a.clone()),
                        signed_c2b_proof: Some(signed_c2b.clone()),
                        signed_c3a_proofs: c3a_proofs,
                        signed_c3b_proofs: c3b_proofs,
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
            proof_type: ProofType::C0PkBfv,
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
                "DKG {:?} proof request failed for E3 {}: {err} — threshold share will not be published without proof",
                kind, e3_id
            );
            self.threshold_correlation
                .retain(|_, (eid, _)| *eid != e3_id);
            self.pending_threshold.remove(&e3_id);
        }

        if let Some((e3_id, kind)) = self.decryption_correlation.remove(msg.correlation_id()) {
            error!(
                "C4 {:?} proof request failed for E3 {}: {err} — DecryptionKeyShared will not be published",
                kind, e3_id
            );
            self.decryption_correlation
                .retain(|_, (eid, _)| *eid != e3_id);
            self.pending_decryption.remove(&e3_id);
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
