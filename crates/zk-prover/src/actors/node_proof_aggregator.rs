// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Node-level DKG proof aggregation: buffer all inner proofs (C0–C4), then run one
//! [`ZkRequest::NodeDkgFold`] when [`ThresholdSharePending`] says the full set is ready.

use std::collections::{BTreeMap, HashMap};

use actix::{Actor, Addr, Context, Handler};
use alloy::primitives::Address;
use alloy::signers::local::PrivateKeySigner;
use e3_events::{
    BusHandle, ComputeRequest, ComputeRequestError, ComputeResponse, ComputeResponseKind,
    CorrelationId, DKGInnerProofReady, DKGRecursiveAggregationComplete, DkgFoldAttestationPayload,
    E3id, EnclaveEvent, EnclaveEventData, EventContext, EventPublisher, EventSubscriber, EventType,
    NodeDkgFoldRequest, Proof, Sequenced, ShareEncryptionProofRequest, SignedDkgFoldAttestation,
    ThresholdSharePending, TypedEvent, ZkRequest, ZkResponse,
};
use e3_fhe_params::build_pair_for_preset;
use tracing::{error, info, warn};

use crate::node_fold_public::extract_node_fold_agg_commits;

/// Metadata from [`ThresholdSharePending`] for slot indices and sizing.
struct NodeDkgFoldMeta {
    party_id: u64,
    total_expected: usize,
    sk_enc_count: usize,
    e_sm_enc_count: usize,
    sk_share_encryption_requests: Vec<ShareEncryptionProofRequest>,
    e_sm_share_encryption_requests: Vec<ShareEncryptionProofRequest>,
    committee_n: usize,
    n_moduli: usize,
    params_preset: e3_fhe_params::BfvPreset,
}

/// Per-E3 collection state: buffer proofs by `seq` until the monolithic fold can run.
struct DkgProofCollectionState {
    meta: NodeDkgFoldMeta,
    buffer: BTreeMap<usize, Proof>,
    fold_correlation: Option<CorrelationId>,
    last_ec: EventContext<Sequenced>,
}

/// Actor that collects DKG inner proofs and dispatches a single [`ZkRequest::NodeDkgFold`].
pub struct NodeProofAggregator {
    bus: BusHandle,
    signer: PrivateKeySigner,
    /// On-chain `DkgFoldAttestationVerifier` address (EIP-712 `verifyingContract`).
    /// `None` is only valid when proof aggregation will never run for this node.
    dkg_fold_attestation_verifier: Option<Address>,
    states: HashMap<E3id, DkgProofCollectionState>,
    fold_correlation: HashMap<CorrelationId, E3id>,
    pending_inner_proofs: HashMap<E3id, BTreeMap<usize, Proof>>,
}

impl NodeProofAggregator {
    pub fn new(
        bus: &BusHandle,
        signer: PrivateKeySigner,
        dkg_fold_attestation_verifier: Option<Address>,
    ) -> Self {
        Self {
            bus: bus.clone(),
            signer,
            dkg_fold_attestation_verifier,
            states: HashMap::new(),
            fold_correlation: HashMap::new(),
            pending_inner_proofs: HashMap::new(),
        }
    }

    pub fn setup(
        bus: &BusHandle,
        signer: PrivateKeySigner,
        dkg_fold_attestation_verifier: Option<Address>,
    ) -> Addr<Self> {
        let addr = Self::new(bus, signer, dkg_fold_attestation_verifier).start();
        bus.subscribe(EventType::ThresholdSharePending, addr.clone().into());
        bus.subscribe(EventType::DKGInnerProofReady, addr.clone().into());
        bus.subscribe(EventType::ComputeResponse, addr.clone().into());
        bus.subscribe(EventType::ComputeRequestError, addr.clone().into());
        addr
    }

    fn handle_threshold_share_pending(&mut self, msg: TypedEvent<ThresholdSharePending>) {
        let (msg, ec) = msg.into_components();
        let e3_id = msg.e3_id.clone();

        if !msg.proof_aggregation_enabled {
            self.pending_inner_proofs.remove(&e3_id);
            info!(
                "NodeProofAggregator: proof aggregation disabled for E3 {} — skipping",
                e3_id
            );
            if let Err(err) = self.bus.publish(
                DKGRecursiveAggregationComplete {
                    e3_id: e3_id.clone(),
                    party_id: msg.full_share.party_id,
                    aggregated_proof: None,
                    fold_attestation: None,
                },
                ec,
            ) {
                error!(
                    "NodeProofAggregator: failed to publish DKGRecursiveAggregationComplete (skipped) for E3 {}: {err}",
                    e3_id
                );
            }
            return;
        }

        let sk_enc_count = msg.sk_share_encryption_requests.len();
        let e_sm_enc_count = msg.e_sm_share_encryption_requests.len();
        let total_expected = 4 + sk_enc_count + e_sm_enc_count + 2;

        let (committee_n, n_moduli) = match build_pair_for_preset(msg.proof_request.params_preset) {
            Ok((threshold_params, _)) => {
                let n = msg.proof_request.committee_size.values().n as usize;
                (n, threshold_params.moduli().len())
            }
            Err(e) => {
                self.pending_inner_proofs.remove(&e3_id);
                error!(
                    "NodeProofAggregator: build_pair_for_preset failed for E3 {}: {e}",
                    e3_id
                );
                return;
            }
        };

        let meta = NodeDkgFoldMeta {
            party_id: msg.full_share.party_id,
            total_expected,
            sk_enc_count,
            e_sm_enc_count,
            sk_share_encryption_requests: msg.sk_share_encryption_requests.clone(),
            e_sm_share_encryption_requests: msg.e_sm_share_encryption_requests.clone(),
            committee_n,
            n_moduli,
            params_preset: msg.proof_request.params_preset,
        };

        info!(
            "NodeProofAggregator: E3 {} party {} — expecting {} inner proofs (C0..C4) for NodeDkgFold",
            e3_id, meta.party_id, total_expected,
        );

        self.initialize_collection_state(e3_id, meta, ec);
    }

    fn handle_inner_proof_ready(&mut self, msg: TypedEvent<DKGInnerProofReady>) {
        let (msg, ec) = msg.into_components();
        let e3_id = msg.e3_id.clone();

        let Some(state) = self.states.get_mut(&e3_id) else {
            let pending = self.pending_inner_proofs.entry(e3_id.clone()).or_default();
            pending.insert(msg.seq, msg.proof);
            warn!(
                "NodeProofAggregator: received DKGInnerProofReady for E3 {} before ThresholdSharePending — prebuffered seq={} (have {})",
                e3_id,
                msg.seq,
                pending.len()
            );
            return;
        };

        if state.fold_correlation.is_some() {
            warn!(
                "NodeProofAggregator: seq={} arrived while NodeDkgFold in flight — dropped",
                msg.seq
            );
            return;
        }

        state.buffer.insert(msg.seq, msg.proof);
        state.last_ec = ec;

        info!(
            "NodeProofAggregator: buffered seq={} for E3 {} (have {}/{})",
            msg.seq,
            e3_id,
            state.buffer.len(),
            state.meta.total_expected
        );

        self.try_dispatch_node_dkg_fold(&e3_id);
    }

    fn initialize_collection_state(
        &mut self,
        e3_id: E3id,
        meta: NodeDkgFoldMeta,
        ec: EventContext<Sequenced>,
    ) {
        let mut buffer = self.pending_inner_proofs.remove(&e3_id).unwrap_or_default();
        if !buffer.is_empty() {
            info!(
                "NodeProofAggregator: recovered {} prebuffered inner proofs for E3 {}",
                buffer.len(),
                e3_id
            );
        }

        self.states.insert(
            e3_id.clone(),
            DkgProofCollectionState {
                meta,
                buffer: std::mem::take(&mut buffer),
                fold_correlation: None,
                last_ec: ec,
            },
        );

        self.try_dispatch_node_dkg_fold(&e3_id);
    }

    fn try_dispatch_node_dkg_fold(&mut self, e3_id: &E3id) {
        let state = match self.states.get_mut(e3_id) {
            Some(s) => s,
            None => return,
        };
        let n = state.meta.total_expected;
        if state.buffer.len() != n || !(0..n).all(|i| state.buffer.contains_key(&i)) {
            return;
        }

        let meta = &state.meta;
        let c3_total_slots = meta.committee_n * meta.n_moduli;
        let slots_a: Vec<u32> = meta
            .sk_share_encryption_requests
            .iter()
            .map(|r| r.c3_slot_index(meta.n_moduli))
            .collect();
        let slots_b: Vec<u32> = meta
            .e_sm_share_encryption_requests
            .iter()
            .map(|r| r.c3_slot_index(meta.n_moduli))
            .collect();

        let sk = meta.sk_enc_count;
        let esm = meta.e_sm_enc_count;
        let buf = &state.buffer;
        let get = |seq: usize| {
            buf.get(&seq)
                .cloned()
                .expect("buffer contains all seq indices")
        };

        let c0_proof = get(0);
        let c1_proof = get(1);
        let c2a_proof = get(2);
        let c2b_proof = get(3);
        let mut c3a_inner_proofs = Vec::with_capacity(sk);
        for s in 0..sk {
            c3a_inner_proofs.push(get(4 + s));
        }
        let mut c3b_inner_proofs = Vec::with_capacity(esm);
        for s in 0..esm {
            c3b_inner_proofs.push(get(4 + sk + s));
        }
        let c4a_seq = 4 + sk + esm;
        let c4a_proof = get(c4a_seq);
        let c4b_proof = get(c4a_seq + 1);

        let corr = CorrelationId::new();
        let ec = state.last_ec.clone();
        let party_id = meta.party_id;
        let preset = meta.params_preset;

        let req = NodeDkgFoldRequest {
            c0_proof,
            c1_proof,
            c2a_proof,
            c2b_proof,
            c3a_inner_proofs,
            c3b_inner_proofs,
            c4a_proof,
            c4b_proof,
            c3_slot_indices_a: slots_a,
            c3_slot_indices_b: slots_b,
            c3_total_slots,
            party_id,
            params_preset: preset,
        };

        state.fold_correlation = Some(corr);
        self.fold_correlation.insert(corr, e3_id.clone());

        info!(
            "NodeProofAggregator: dispatching NodeDkgFold for E3 {} party {}",
            e3_id, party_id
        );

        if let Err(err) = self.bus.publish(
            ComputeRequest::zk(ZkRequest::NodeDkgFold(req), corr, e3_id.clone()),
            ec,
        ) {
            error!(
                "NodeProofAggregator: failed to publish NodeDkgFold for E3 {}: {err}",
                e3_id
            );
            let _ = self.states.get_mut(e3_id).map(|s| {
                s.fold_correlation = None;
            });
            self.fold_correlation.remove(&corr);
        }
    }

    fn handle_node_dkg_response(&mut self, correlation_id: &CorrelationId, proof: Proof) {
        let Some(e3_id) = self.fold_correlation.remove(correlation_id) else {
            return;
        };

        let Some(state) = self.states.remove(&e3_id) else {
            error!(
                "NodeProofAggregator: NodeDkgFold response for unknown E3 {}",
                e3_id
            );
            return;
        };

        let party_id = state.meta.party_id;
        let committee_n = state.meta.committee_n;
        let committee_h = committee_n;
        let n_moduli = state.meta.n_moduli;

        let fold_attestation = match extract_node_fold_agg_commits(
            &proof,
            committee_n,
            committee_h,
            n_moduli,
        ) {
            Ok((extracted_party, commits)) => {
                if extracted_party != party_id {
                    error!(
                        e3_id = %e3_id,
                        expected_party_id = party_id,
                        extracted_party_id = extracted_party,
                        "NodeFold public party_id does not match sortition party_id"
                    );
                    None
                } else if let Some(verifying_contract) = self.dkg_fold_attestation_verifier {
                    let payload = DkgFoldAttestationPayload {
                        e3_id: e3_id.clone(),
                        verifying_contract,
                        party_id,
                        agg_commits: commits,
                    };
                    match SignedDkgFoldAttestation::sign(payload, &self.signer) {
                        Ok(signed) => Some(signed),
                        Err(e) => {
                            error!(
                                e3_id = %e3_id,
                                party_id,
                                error = %e,
                                "failed to sign DkgFoldAttestation"
                            );
                            None
                        }
                    }
                } else {
                    error!(
                        e3_id = %e3_id,
                        party_id,
                        "NodeProofAggregator: cannot sign DkgFoldAttestation — `dkg_fold_attestation_verifier` address not configured"
                    );
                    None
                }
            }
            Err(e) => {
                error!(
                    e3_id = %e3_id,
                    party_id,
                    error = %e,
                    "failed to extract sk_agg/esm_agg from NodeFold proof"
                );
                None
            }
        };

        if fold_attestation.is_none() {
            error!(
                e3_id = %e3_id,
                party_id,
                "NodeDkgFold succeeded but fold attestation missing — publishing without proof"
            );
            if let Err(err) = self.bus.publish(
                DKGRecursiveAggregationComplete {
                    e3_id: e3_id.clone(),
                    party_id,
                    aggregated_proof: None,
                    fold_attestation: None,
                },
                state.last_ec,
            ) {
                error!(
                    "NodeProofAggregator: failed to publish DKGRecursiveAggregationComplete for E3 {}: {err}",
                    e3_id
                );
            }
            return;
        }

        info!(
            "NodeProofAggregator: NodeDkgFold complete for E3 {} party {} — publishing DKGRecursiveAggregationComplete",
            e3_id, party_id
        );

        if let Err(err) = self.bus.publish(
            DKGRecursiveAggregationComplete {
                e3_id: e3_id.clone(),
                party_id,
                aggregated_proof: Some(proof),
                fold_attestation,
            },
            state.last_ec,
        ) {
            error!(
                "NodeProofAggregator: failed to publish DKGRecursiveAggregationComplete for E3 {}: {err}",
                e3_id
            );
        }
    }
}

impl Actor for NodeProofAggregator {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for NodeProofAggregator {
    type Result = ();

    fn handle(&mut self, msg: EnclaveEvent, _ctx: &mut Self::Context) -> Self::Result {
        let (data, ec) = msg.into_components();
        match data {
            EnclaveEventData::ThresholdSharePending(data) => {
                self.handle_threshold_share_pending(TypedEvent::new(data, ec));
            }
            EnclaveEventData::DKGInnerProofReady(data) => {
                self.handle_inner_proof_ready(TypedEvent::new(data, ec));
            }
            EnclaveEventData::ComputeResponse(data) => {
                self.handle_compute_response(TypedEvent::new(data, ec));
            }
            EnclaveEventData::ComputeRequestError(data) => {
                self.handle_compute_request_error(TypedEvent::new(data, ec));
            }
            _ => {}
        }
    }
}

impl Handler<TypedEvent<ThresholdSharePending>> for NodeProofAggregator {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ThresholdSharePending>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_threshold_share_pending(msg);
    }
}

impl Handler<TypedEvent<DKGInnerProofReady>> for NodeProofAggregator {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<DKGInnerProofReady>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_inner_proof_ready(msg);
    }
}

impl Handler<TypedEvent<ComputeResponse>> for NodeProofAggregator {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ComputeResponse>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_compute_response(msg);
    }
}

impl Handler<TypedEvent<ComputeRequestError>> for NodeProofAggregator {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ComputeRequestError>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        self.handle_compute_request_error(msg);
    }
}

impl NodeProofAggregator {
    fn handle_compute_response(&mut self, msg: TypedEvent<ComputeResponse>) {
        let (msg, _ec) = msg.into_components();
        if let ComputeResponseKind::Zk(ZkResponse::NodeDkgFold(resp)) = msg.response {
            self.handle_node_dkg_response(&msg.correlation_id, resp.proof);
        }
    }

    fn handle_compute_request_error(&mut self, msg: TypedEvent<ComputeRequestError>) {
        let (msg, ec) = msg.into_components();
        if let Some(e3_id) = self.fold_correlation.remove(msg.correlation_id()) {
            error!(
                "NodeProofAggregator: NodeDkgFold failed for E3 {}: {:?} — aggregation aborted",
                e3_id,
                msg.get_err()
            );
            let state = self.states.remove(&e3_id);
            warn!(
                "NodeProofAggregator: E3 {} NodeDkgFold failed — publishing DKGRecursiveAggregationComplete(None)",
                e3_id
            );

            if let Some(state) = state {
                if let Err(err) = self.bus.publish(
                    DKGRecursiveAggregationComplete {
                        e3_id: e3_id.clone(),
                        party_id: state.meta.party_id,
                        aggregated_proof: None,
                        fold_attestation: None,
                    },
                    ec,
                ) {
                    error!(
                        "NodeProofAggregator: failed to publish DKGRecursiveAggregationComplete(None) for E3 {}: {err}",
                        e3_id
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::Result;
    use e3_events::{
        CircuitName, ComputeRequestErrorKind, ComputeRequestKind, Event, HistoryCollector,
        TakeEvents, Unsequenced, ZkError,
    };
    use e3_test_helpers::get_common_setup;

    fn test_ctx(data: impl Into<EnclaveEventData>) -> EventContext<Sequenced> {
        EventContext::<Unsequenced>::from(data.into()).sequence(0)
    }

    fn test_signer() -> PrivateKeySigner {
        "0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80"
            .parse()
            .expect("test signer")
    }

    fn dummy_proof(seed: u8) -> Proof {
        Proof::new(
            CircuitName::PkAggregation,
            e3_utils::ArcBytes::from_bytes(&[seed]),
            e3_utils::ArcBytes::from_bytes(&[seed.wrapping_add(1)]),
        )
    }

    async fn next_event(history: &Addr<HistoryCollector<EnclaveEvent>>) -> Result<EnclaveEvent> {
        let mut result = history.send(TakeEvents::<EnclaveEvent>::new(1)).await?;
        assert!(!result.timed_out, "timed out waiting for an event");
        Ok(result.events.pop().expect("expected one event"))
    }

    #[actix::test]
    async fn node_dkg_fold_compute_error_emits_none_aggregation_result() -> Result<()> {
        let (bus, _rng, _seed, _params, _crp, _errors, history) = get_common_setup(None)?;
        let mut aggregator = NodeProofAggregator::new(&bus, test_signer(), None);
        let e3_id = E3id::new("42", 1);
        let correlation_id = CorrelationId::new();

        aggregator.states.insert(
            e3_id.clone(),
            DkgProofCollectionState {
                meta: NodeDkgFoldMeta {
                    party_id: 7,
                    total_expected: 0,
                    sk_enc_count: 0,
                    e_sm_enc_count: 0,
                    sk_share_encryption_requests: Vec::new(),
                    e_sm_share_encryption_requests: Vec::new(),
                    committee_n: 0,
                    n_moduli: 0,
                    params_preset: e3_fhe_params::BfvPreset::InsecureThreshold512,
                },
                buffer: BTreeMap::new(),
                fold_correlation: Some(correlation_id),
                last_ec: test_ctx(DKGRecursiveAggregationComplete {
                    e3_id: e3_id.clone(),
                    party_id: 7,
                    aggregated_proof: None,
                    fold_attestation: None,
                }),
            },
        );
        aggregator
            .fold_correlation
            .insert(correlation_id, e3_id.clone());

        let request = ComputeRequest::zk(
            ZkRequest::NodeDkgFold(NodeDkgFoldRequest {
                c0_proof: dummy_proof(1),
                c1_proof: dummy_proof(2),
                c2a_proof: dummy_proof(3),
                c2b_proof: dummy_proof(4),
                c3a_inner_proofs: Vec::new(),
                c3b_inner_proofs: Vec::new(),
                c4a_proof: dummy_proof(5),
                c4b_proof: dummy_proof(6),
                c3_slot_indices_a: Vec::new(),
                c3_slot_indices_b: Vec::new(),
                c3_total_slots: 0,
                party_id: 7,
                params_preset: e3_fhe_params::BfvPreset::InsecureThreshold512,
            }),
            correlation_id,
            e3_id.clone(),
        );

        aggregator.handle_compute_request_error(TypedEvent::new(
            ComputeRequestError::new(
                ComputeRequestErrorKind::Zk(ZkError::ProofGenerationFailed("boom".to_string())),
                request,
            ),
            test_ctx(DKGRecursiveAggregationComplete {
                e3_id: e3_id.clone(),
                party_id: 7,
                aggregated_proof: None,
                fold_attestation: None,
            }),
        ));

        let event = next_event(&history).await?;
        assert!(matches!(
            event.into_data(),
            EnclaveEventData::DKGRecursiveAggregationComplete(data)
                if data.e3_id == e3_id
                    && data.party_id == 7
                    && data.aggregated_proof.is_none()
        ));
        assert!(!aggregator.states.contains_key(&e3_id));
        assert!(aggregator.fold_correlation.is_empty());

        Ok(())
    }

    #[actix::test]
    async fn early_inner_proof_is_prebuffered_until_collection_starts() -> Result<()> {
        let (bus, _rng, _seed, _params, _crp, _errors, history) = get_common_setup(None)?;
        let mut aggregator = NodeProofAggregator::new(&bus, test_signer(), None);
        let e3_id = E3id::new("43", 1);
        let early_proof = dummy_proof(10);

        aggregator.handle_inner_proof_ready(TypedEvent::new(
            DKGInnerProofReady {
                e3_id: e3_id.clone(),
                party_id: 7,
                proof: early_proof.clone(),
                seq: 0,
            },
            test_ctx(DKGInnerProofReady {
                e3_id: e3_id.clone(),
                party_id: 7,
                proof: early_proof.clone(),
                seq: 0,
            }),
        ));

        assert_eq!(
            aggregator
                .pending_inner_proofs
                .get(&e3_id)
                .map(BTreeMap::len),
            Some(1)
        );

        aggregator.initialize_collection_state(
            e3_id.clone(),
            NodeDkgFoldMeta {
                party_id: 7,
                total_expected: 6,
                sk_enc_count: 0,
                e_sm_enc_count: 0,
                sk_share_encryption_requests: Vec::new(),
                e_sm_share_encryption_requests: Vec::new(),
                committee_n: 0,
                n_moduli: 0,
                params_preset: e3_fhe_params::BfvPreset::InsecureThreshold512,
            },
            test_ctx(DKGRecursiveAggregationComplete {
                e3_id: e3_id.clone(),
                party_id: 7,
                aggregated_proof: None,
                fold_attestation: None,
            }),
        );

        assert!(!aggregator.pending_inner_proofs.contains_key(&e3_id));
        assert_eq!(
            aggregator
                .states
                .get(&e3_id)
                .map(|state| state.buffer.len()),
            Some(1)
        );

        for seq in 1..6 {
            let proof = dummy_proof((10 + seq) as u8);
            aggregator.handle_inner_proof_ready(TypedEvent::new(
                DKGInnerProofReady {
                    e3_id: e3_id.clone(),
                    party_id: 7,
                    proof: proof.clone(),
                    seq,
                },
                test_ctx(DKGInnerProofReady {
                    e3_id: e3_id.clone(),
                    party_id: 7,
                    proof,
                    seq,
                }),
            ));
        }

        let event = next_event(&history).await?;
        match event.into_data() {
            EnclaveEventData::ComputeRequest(request) => {
                assert_eq!(request.e3_id, e3_id);
                match request.request {
                    ComputeRequestKind::Zk(ZkRequest::NodeDkgFold(fold_request)) => {
                        assert_eq!(fold_request.c0_proof, early_proof);
                    }
                    other => panic!("expected NodeDkgFold request, got {other:?}"),
                }
            }
            other => panic!("expected ComputeRequest event, got {other:?}"),
        }

        assert!(aggregator
            .states
            .get(&e3_id)
            .and_then(|state| state.fold_correlation)
            .is_some());

        Ok(())
    }
}
