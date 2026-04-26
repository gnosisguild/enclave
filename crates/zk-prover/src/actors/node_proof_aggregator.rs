// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Node-level DKG proof aggregation: buffer all inner proofs (C0–C4), then run one
//! [`ZkRequest::NodeDkgFold`] when [`ThresholdSharePending`] says the full set is ready.

use std::collections::{BTreeMap, HashMap};

use actix::{Actor, Addr, Context, Handler};
use e3_events::{
    BusHandle, ComputeRequest, ComputeRequestError, ComputeResponse, ComputeResponseKind,
    CorrelationId, DKGInnerProofReady, DKGRecursiveAggregationComplete, E3id, EnclaveEvent,
    EnclaveEventData, EventContext, EventPublisher, EventSubscriber, EventType, NodeDkgFoldRequest,
    Proof, Sequenced, ShareEncryptionProofRequest, ThresholdSharePending, TypedEvent, ZkRequest,
    ZkResponse,
};
use e3_fhe_params::build_pair_for_preset;
use tracing::{error, info, warn};

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
    states: HashMap<E3id, DkgProofCollectionState>,
    fold_correlation: HashMap<CorrelationId, E3id>,
}

impl NodeProofAggregator {
    pub fn new(bus: &BusHandle) -> Self {
        Self {
            bus: bus.clone(),
            states: HashMap::new(),
            fold_correlation: HashMap::new(),
        }
    }

    pub fn setup(bus: &BusHandle) -> Addr<Self> {
        let addr = Self::new(bus).start();
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
            info!(
                "NodeProofAggregator: proof aggregation disabled for E3 {} — skipping",
                e3_id
            );
            if let Err(err) = self.bus.publish(
                DKGRecursiveAggregationComplete {
                    e3_id: e3_id.clone(),
                    party_id: msg.full_share.party_id,
                    aggregated_proof: None,
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

        self.states.insert(
            e3_id.clone(),
            DkgProofCollectionState {
                meta,
                buffer: BTreeMap::new(),
                fold_correlation: None,
                last_ec: ec,
            },
        );
    }

    fn handle_inner_proof_ready(&mut self, msg: TypedEvent<DKGInnerProofReady>) {
        let (msg, ec) = msg.into_components();
        let e3_id = msg.e3_id.clone();

        let Some(state) = self.states.get_mut(&e3_id) else {
            error!(
                "NodeProofAggregator: received DKGInnerProofReady for E3 {} before ThresholdSharePending — proof dropped",
                e3_id
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

        info!(
            "NodeProofAggregator: NodeDkgFold complete for E3 {} party {} — publishing DKGRecursiveAggregationComplete",
            e3_id, state.meta.party_id
        );

        if let Err(err) = self.bus.publish(
            DKGRecursiveAggregationComplete {
                e3_id: e3_id.clone(),
                party_id: state.meta.party_id,
                aggregated_proof: Some(proof),
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
        let (msg, _ec) = msg.into_components();
        if let Some(e3_id) = self.fold_correlation.remove(msg.correlation_id()) {
            error!(
                "NodeProofAggregator: NodeDkgFold failed for E3 {}: {:?} — aggregation aborted",
                e3_id,
                msg.get_err()
            );
            self.states.remove(&e3_id);
            warn!(
                "NodeProofAggregator: E3 {} aggregation state discarded due to error",
                e3_id
            );
        }
    }
}
