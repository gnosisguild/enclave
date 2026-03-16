// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

//! Incremental node-level proof aggregation for DKG.
//!
//! `NodeProofAggregator` subscribes to `ThresholdSharePending` to learn how
//! many proofs to expect per E3, then folds each `DKGInnerProofReady` wrapped
//! proof into a running aggregate in strict `seq` order, using a reorder
//! buffer for out-of-order arrivals.
//!
//! When all expected proofs have been folded, it publishes
//! `DKGRecursiveAggregationComplete`.
//!
//! Ordering guarantee: `ThresholdSharePending` is always published to the bus
//! before any `DKGInnerProofReady` for the same E3 (ProofRequestActor holds
//! back C0's event until after `ThresholdSharePending` is processed).
//! Arriving out of that order is treated as a programming error and logged.

use std::collections::{BTreeMap, HashMap};

use actix::{Actor, Addr, Context, Handler};
use e3_events::{
    BusHandle, ComputeRequest, ComputeRequestError, ComputeResponse, ComputeResponseKind,
    CorrelationId, DKGInnerProofReady, DKGRecursiveAggregationComplete, E3id, EnclaveEvent,
    EnclaveEventData, EventContext, EventPublisher, EventSubscriber, EventType, Proof, Sequenced,
    ThresholdSharePending, TypedEvent, ZkRequest, ZkResponse,
};
use tracing::{error, info, warn};

/// Per-E3 rolling aggregation state for one node's proofs.
struct RollingAggregationState {
    party_id: u64,
    /// Total proofs expected (for progress logging).
    total_expected: usize,
    /// Proofs buffered out-of-order, keyed by seq index.
    buffer: BTreeMap<usize, Proof>,
    /// The running accumulated (folded) proof.
    accumulated: Option<Proof>,
    /// Next seq index expected for folding.
    next_to_aggregate: usize,
    /// Number of proofs remaining to process (decrements on first store + each fold completion).
    remaining: usize,
    /// Correlation ID for the in-flight fold, if any.
    fold_correlation: Option<CorrelationId>,
    /// EventContext for publishing.
    last_ec: EventContext<Sequenced>,
}

/// Actor that incrementally folds DKG inner proofs into a single node-level proof.
pub struct NodeProofAggregator {
    bus: BusHandle,
    states: HashMap<E3id, RollingAggregationState>,
    /// Reverse map: fold correlation_id -> e3_id.
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

        let sk_enc_count = msg.sk_share_encryption_requests.len();
        let e_sm_enc_count = msg.e_sm_share_encryption_requests.len();
        // Must mirror the formula in ProofRequestActor::handle_threshold_share_pending:
        //   C0 + C1 + C2a + C2b + C3a×sk_enc + C3b×e_sm_enc + C4a + C4b
        let total_expected = 4 + sk_enc_count + e_sm_enc_count + 2;

        self.states.entry(e3_id.clone()).or_insert_with(|| {
            info!(
                "NodeProofAggregator: initializing state for E3 {} party {} (total_expected={}, ~{} fold steps)",
                e3_id, msg.full_share.party_id, total_expected,
                total_expected.saturating_sub(1),
            );
            RollingAggregationState {
                party_id: msg.full_share.party_id,
                total_expected,
                buffer: BTreeMap::new(),
                accumulated: None,
                next_to_aggregate: 0,
                remaining: total_expected,
                fold_correlation: None,
                last_ec: ec,
            }
        });
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

        state.buffer.insert(msg.seq, msg.wrapped_proof);
        state.last_ec = ec;

        info!(
            "NodeProofAggregator: buffered seq={} for E3 {} (remaining={})",
            msg.seq, e3_id, state.remaining
        );

        self.try_advance(&e3_id);
    }

    fn try_advance(&mut self, e3_id: &E3id) {
        loop {
            let state = match self.states.get_mut(e3_id) {
                Some(s) => s,
                None => return,
            };

            if state.fold_correlation.is_some() || state.remaining == 0 {
                return;
            }

            let next_proof = match state.buffer.remove(&state.next_to_aggregate) {
                Some(p) => p,
                None => return, // not yet available
            };

            if state.accumulated.is_none() {
                // First proof: store as accumulated
                info!(
                    "NodeProofAggregator: storing first proof (seq={}) for E3 {}",
                    state.next_to_aggregate, e3_id
                );
                state.accumulated = Some(next_proof);
                state.remaining -= 1;
                state.next_to_aggregate += 1;

                if state.remaining == 0 {
                    // Only one proof total — done immediately
                    self.publish_complete(e3_id);
                    return;
                }
            } else {
                // Fold accumulated + next_proof
                let acc = state.accumulated.take().expect("checked above");
                let acc_restore = acc.clone();
                let next_proof_restore = next_proof.clone();
                let seq = state.next_to_aggregate;
                let corr = CorrelationId::new();
                let ec = state.last_ec.clone();
                let e3_id_clone = e3_id.clone();

                let folds_completed = state.total_expected - state.remaining - 1;
                let total_folds = state.total_expected.saturating_sub(1);
                info!(
                    "NodeProofAggregator: dispatching fold step {}/{} (seq={}) for E3 {}",
                    folds_completed + 1,
                    total_folds,
                    seq,
                    e3_id
                );

                match self.bus.publish(
                    ComputeRequest::zk(
                        ZkRequest::FoldProofs {
                            proof1: acc,
                            proof2: next_proof,
                            target_evm: false,
                        },
                        corr,
                        e3_id_clone,
                    ),
                    ec,
                ) {
                    Ok(()) => {
                        state.fold_correlation = Some(corr);
                        state.next_to_aggregate += 1;
                        self.fold_correlation.insert(corr, e3_id.clone());
                    }
                    Err(err) => {
                        error!(
                            "NodeProofAggregator: failed to publish fold request for E3 {}: {err}",
                            e3_id
                        );
                        state.accumulated = Some(acc_restore);
                        state.buffer.insert(seq, next_proof_restore);
                    }
                }

                return; // wait for fold response
            }
        }
    }

    fn handle_fold_response(&mut self, correlation_id: &CorrelationId, proof: Proof) {
        let Some(e3_id) = self.fold_correlation.remove(correlation_id) else {
            return;
        };

        let Some(state) = self.states.get_mut(&e3_id) else {
            error!(
                "NodeProofAggregator: received fold response for unknown E3 {}",
                e3_id
            );
            return;
        };

        state.remaining -= 1;
        let folds_completed = state.total_expected - state.remaining - 1;
        let total_folds = state.total_expected.saturating_sub(1);
        info!(
            "NodeProofAggregator: fold step {}/{} complete for E3 {} ({} proofs remaining)",
            folds_completed, total_folds, e3_id, state.remaining
        );

        state.accumulated = Some(proof);
        state.fold_correlation = None;

        if state.remaining == 0 {
            self.publish_complete(&e3_id);
        } else {
            self.try_advance(&e3_id);
        }
    }

    fn publish_complete(&mut self, e3_id: &E3id) {
        let Some(state) = self.states.remove(e3_id) else {
            return;
        };

        let Some(aggregated_proof) = state.accumulated else {
            error!(
                "NodeProofAggregator: no accumulated proof for E3 {} at completion",
                e3_id
            );
            return;
        };

        info!(
            "NodeProofAggregator: all proofs folded for E3 {} party {} — publishing DKGRecursiveAggregationComplete",
            e3_id, state.party_id
        );

        if let Err(err) = self.bus.publish(
            DKGRecursiveAggregationComplete {
                e3_id: e3_id.clone(),
                party_id: state.party_id,
                aggregated_proof,
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
        if let ComputeResponseKind::Zk(ZkResponse::FoldProofs(resp)) = msg.response {
            self.handle_fold_response(&msg.correlation_id, resp.proof);
        }
    }

    fn handle_compute_request_error(&mut self, msg: TypedEvent<ComputeRequestError>) {
        let (msg, _ec) = msg.into_components();
        if let Some(e3_id) = self.fold_correlation.remove(msg.correlation_id()) {
            error!(
                "NodeProofAggregator: fold request failed for E3 {}: {:?} — aggregation aborted",
                e3_id,
                msg.get_err()
            );
            self.states.remove(&e3_id);
            warn!(
                "NodeProofAggregator: E3 {} aggregation state discarded due to fold error",
                e3_id
            );
        }
    }
}
