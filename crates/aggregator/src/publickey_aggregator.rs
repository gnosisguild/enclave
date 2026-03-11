// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::proof_fold::ProofFoldState;
use actix::prelude::*;
use anyhow::Result;
use e3_bfv_client::client::compute_pk_commitment;
use e3_data::Persistable;
use e3_events::{
    prelude::*, BusHandle, ComputeResponse, ComputeResponseKind, DKGRecursiveAggregationComplete,
    Die, E3id, EnclaveEvent, EnclaveEventData, EventContext, KeyshareCreated, OrderedSet,
    PartyProofsToVerify, PkAggregationProofPending, PkAggregationProofRequest,
    PkAggregationProofSigned, Proof, PublicKeyAggregated, Seed, Sequenced,
    ShareVerificationComplete, ShareVerificationDispatched, SignedProofPayload, TypedEvent,
    VerificationKind, ZkResponse,
};
use e3_events::{trap, EType};
use e3_fhe::{Fhe, GetAggregatePublicKey};
use e3_fhe_params::BfvPreset;
use e3_utils::NotifySync;
use e3_utils::{ArcBytes, MAILBOX_LIMIT};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use tracing::{error, info, warn};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PublicKeyAggregatorState {
    Collecting {
        threshold_n: usize,
        threshold_m: usize,
        keyshares: OrderedSet<ArcBytes>,
        /// C1 proofs collected from KeyshareCreated events, indexed by insertion order.
        c1_proofs: Vec<Option<SignedProofPayload>>,
        seed: Seed,
        nodes: OrderedSet<String>,
        /// Insertion-ordered (node, keyshare) pairs.
        /// Index matches `c1_proofs`, giving the party ID for verification.
        #[serde(default)]
        submission_order: Vec<(String, ArcBytes)>,
    },
    VerifyingC1 {
        /// Insertion-ordered (node, keyshare) pairs from Collecting.
        /// Index matches `c1_proofs`, giving the party ID used in verification.
        submission_order: Vec<(String, ArcBytes)>,
        threshold_m: usize,
        /// C1 proofs in the same insertion order as `submission_order`.
        c1_proofs: Vec<Option<SignedProofPayload>>,
        /// Party indices that submitted no C1 proof — treated as dishonest.
        no_proof_parties: Vec<u64>,
    },
    GeneratingC5Proof {
        public_key: ArcBytes,
        public_key_hash: [u8; 32],
        keyshare_bytes: Vec<ArcBytes>,
        nodes: OrderedSet<String>,
    },
    Complete {
        public_key: ArcBytes,
        keyshares: OrderedSet<ArcBytes>,
        nodes: OrderedSet<String>,
    },
}

impl PublicKeyAggregatorState {
    pub fn init(threshold_n: usize, threshold_m: usize, seed: Seed) -> Self {
        PublicKeyAggregatorState::Collecting {
            threshold_n,
            threshold_m,
            keyshares: OrderedSet::new(),
            c1_proofs: Vec::new(),
            seed,
            nodes: OrderedSet::new(),
            submission_order: Vec::new(),
        }
    }
}

pub struct PublicKeyAggregator {
    fhe: Arc<Fhe>,
    bus: BusHandle,
    e3_id: E3id,
    state: Persistable<PublicKeyAggregatorState>,
    params_preset: BfvPreset,
    /// Buffer for per-node DKG recursive proofs keyed by party_id.
    dkg_node_proofs: HashMap<u64, Proof>,
    /// Party IDs that passed C1 verification (set at C1 completion).
    honest_party_ids: Option<BTreeSet<u64>>,
    /// Party IDs excluded as dishonest after C1 verification.
    dishonest_parties: BTreeSet<u64>,
    /// Cross-node DKG proof fold state.
    cross_node_fold: ProofFoldState,
    /// C5 proof stored while waiting for cross-node fold completion.
    c5_proof_pending: Option<Proof>,
    /// Last event context for publishing.
    last_ec: Option<EventContext<Sequenced>>,
}

pub struct PublicKeyAggregatorParams {
    pub fhe: Arc<Fhe>,
    pub bus: BusHandle,
    pub e3_id: E3id,
    pub params_preset: BfvPreset,
}

/// Aggregate PublicKey for a committee of nodes. This actor listens for KeyshareCreated events
/// around a particular e3_id, verifies C1 proofs, aggregates the public key, generates a C5
/// proof of correct aggregation, and broadcasts a PublicKeyAggregated event on the event bus.
impl PublicKeyAggregator {
    pub fn new(
        params: PublicKeyAggregatorParams,
        state: Persistable<PublicKeyAggregatorState>,
    ) -> Self {
        PublicKeyAggregator {
            fhe: params.fhe,
            bus: params.bus,
            e3_id: params.e3_id,
            state,
            params_preset: params.params_preset,
            dkg_node_proofs: HashMap::new(),
            honest_party_ids: None,
            dishonest_parties: BTreeSet::new(),
            cross_node_fold: ProofFoldState::new(),
            c5_proof_pending: None,
            last_ec: None,
        }
    }

    pub fn add_keyshare(
        &mut self,
        keyshare: ArcBytes,
        node: String,
        c1_proof: Option<SignedProofPayload>,
        ec: &EventContext<Sequenced>,
    ) -> Result<()> {
        self.state.try_mutate(&ec, |mut state| {
            let PublicKeyAggregatorState::Collecting {
                threshold_n,
                threshold_m,
                keyshares,
                c1_proofs,
                nodes,
                submission_order,
                ..
            } = &mut state
            else {
                return Err(anyhow::anyhow!("Can only add keyshare in Collecting state"));
            };

            keyshares.insert(keyshare.clone());
            c1_proofs.push(c1_proof);
            nodes.insert(node.clone());
            submission_order.push((node, keyshare));
            let n = *threshold_n;
            let m = *threshold_m;
            info!(
                "PublicKeyAggregator got keyshares {}/{}",
                keyshares.len(),
                n
            );
            if keyshares.len() == n {
                info!("All keyshares collected, transitioning to VerifyingC1...");
                return Ok(PublicKeyAggregatorState::VerifyingC1 {
                    submission_order: std::mem::take(submission_order),
                    threshold_m: m,
                    c1_proofs: std::mem::take(c1_proofs),
                    no_proof_parties: Vec::new(),
                });
            }

            Ok(state)
        })
    }

    fn dispatch_c1_verification(
        &mut self,
        c1_proofs: &[Option<SignedProofPayload>],
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
        let mut party_proofs = Vec::new();
        let mut no_proof_parties = Vec::new();

        for (idx, proof_opt) in c1_proofs.iter().enumerate() {
            match proof_opt {
                Some(proof) => {
                    party_proofs.push(PartyProofsToVerify {
                        sender_party_id: idx as u64,
                        signed_proofs: vec![proof.clone()],
                    });
                }
                None => {
                    warn!(
                        "Party {} submitted keyshare without C1 proof — treating as dishonest",
                        idx
                    );
                    no_proof_parties.push(idx as u64);
                }
            }
        }

        // Store no-proof parties in state for the response handler
        if !no_proof_parties.is_empty() {
            self.state.try_mutate(&ec, |mut state| {
                if let PublicKeyAggregatorState::VerifyingC1 {
                    no_proof_parties: ref mut stored,
                    ..
                } = state
                {
                    *stored = no_proof_parties.clone();
                }
                Ok(state)
            })?;
        }

        if party_proofs.is_empty() {
            return Err(anyhow::anyhow!(
                "No C1 proofs to verify — all keyshares must include a signed C1 proof"
            ));
        }

        info!(
            "Dispatching C1 proof verification for {} parties ({} missing proofs)",
            party_proofs.len(),
            no_proof_parties.len()
        );

        self.bus.publish(
            ShareVerificationDispatched {
                e3_id: self.e3_id.clone(),
                kind: VerificationKind::PkGenerationProofs,
                share_proofs: party_proofs,
                decryption_proofs: vec![],
                pre_dishonest: no_proof_parties.into_iter().collect(),
            },
            ec,
        )?;
        Ok(())
    }

    fn handle_c1_verification_complete(
        &mut self,
        msg: TypedEvent<ShareVerificationComplete>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();

        if msg.kind != VerificationKind::PkGenerationProofs {
            return Ok(());
        }

        if msg.e3_id != self.e3_id {
            return Ok(());
        }

        let PublicKeyAggregatorState::VerifyingC1 {
            submission_order,
            threshold_m,
            ..
        } = self
            .state
            .get()
            .ok_or_else(|| anyhow::anyhow!("Expected VerifyingC1 state"))?
        else {
            return Err(anyhow::anyhow!(
                "handle_c1_verification_complete called outside VerifyingC1 state"
            ));
        };

        let dishonest_parties = &msg.dishonest_parties;
        let total_parties = submission_order.len();

        // Filter out dishonest parties using submission_order (insertion-order indexed,
        // matching the party IDs sent to dispatch_c1_verification).
        let (honest_keyshares, honest_nodes): (Vec<ArcBytes>, Vec<String>) = submission_order
            .into_iter()
            .enumerate()
            .filter(|(idx, _)| !dishonest_parties.contains(&(*idx as u64)))
            .map(|(_, (node, ks))| (ks, node))
            .unzip();

        if !dishonest_parties.is_empty() {
            warn!(
                "Filtered out {} dishonest parties from C1 verification: {:?}",
                dishonest_parties.len(),
                dishonest_parties
            );
        }
        self.dishonest_parties = dishonest_parties.clone();

        let honest_party_ids: BTreeSet<u64> = (0..total_parties as u64)
            .filter(|id| !dishonest_parties.contains(id))
            .collect();
        self.honest_party_ids = Some(honest_party_ids);

        // Need at least threshold + 1 honest parties for aggregation
        if honest_keyshares.len() <= threshold_m {
            return Err(anyhow::anyhow!(
                "Not enough honest parties after C1 verification: {} (need at least {})",
                honest_keyshares.len(),
                threshold_m + 1
            ));
        }

        // Synchronous aggregation
        info!(
            "Aggregating public key from {} honest shares...",
            honest_keyshares.len()
        );
        let honest_keyshares_set = OrderedSet::from(honest_keyshares.clone());
        let pubkey = self.fhe.get_aggregate_public_key(GetAggregatePublicKey {
            keyshares: honest_keyshares_set.clone(),
        })?;

        let public_key_hash = compute_pk_commitment(
            pubkey.clone(),
            self.fhe.params.degree(),
            self.fhe.params.plaintext(),
            self.fhe.params.moduli().to_vec(),
        )?;

        let honest_nodes_set = OrderedSet::from(honest_nodes);

        // Publish pending event before transitioning state so a publish
        // failure leaves us in VerifyingC1 (retryable) rather than
        // GeneratingC5Proof (no retry path).
        let committee_h = honest_keyshares.len();

        info!("Publishing PkAggregationProofPending for C5 proof generation...");
        let pubkey = ArcBytes::from_bytes(&pubkey);
        self.bus.publish(
            PkAggregationProofPending {
                e3_id: self.e3_id.clone(),
                proof_request: PkAggregationProofRequest {
                    keyshare_bytes: honest_keyshares.clone(),
                    aggregated_pk_bytes: pubkey.clone(),
                    params_preset: self.params_preset.clone(),
                    // this field is not really used in the circuit, we only use H
                    committee_n: committee_h,
                    committee_h,
                    // this field is not really used in the circuit, we only use H
                    committee_threshold: 0,
                },
                public_key: pubkey.clone(),
                public_key_hash,
                nodes: honest_nodes_set.clone(),
            },
            ec.clone(),
        )?;

        // Transition to GeneratingC5Proof
        self.state.try_mutate(&ec, |_| {
            Ok(PublicKeyAggregatorState::GeneratingC5Proof {
                public_key: pubkey.clone(),
                public_key_hash,
                keyshare_bytes: honest_keyshares,
                nodes: honest_nodes_set,
            })
        })?;

        self.last_ec = Some(ec.clone());
        self.try_start_cross_node_fold(&ec)?;

        Ok(())
    }

    fn handle_pk_aggregation_proof_signed(
        &mut self,
        msg: TypedEvent<PkAggregationProofSigned>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();

        if msg.e3_id != self.e3_id {
            return Ok(());
        }

        if !matches!(
            self.state.get(),
            Some(PublicKeyAggregatorState::GeneratingC5Proof { .. })
        ) {
            return Err(anyhow::anyhow!(
                "handle_pk_aggregation_proof_signed called outside GeneratingC5Proof state"
            ));
        }

        info!("C5 proof signed — waiting for cross-node DKG fold to complete...");

        self.c5_proof_pending = Some(msg.signed_proof.payload.proof);
        self.last_ec = Some(ec);
        self.try_publish_complete()
    }

    // -- Cross-node DKG proof aggregation --------------------------------------------------

    fn handle_dkg_recursive_aggregation_complete(
        &mut self,
        msg: TypedEvent<DKGRecursiveAggregationComplete>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();

        if msg.e3_id != self.e3_id {
            return Ok(());
        }

        if self.dkg_node_proofs.contains_key(&msg.party_id) {
            warn!(
                "Duplicate DKGRecursiveAggregationComplete for party {} — ignoring",
                msg.party_id
            );
            return Ok(());
        }

        info!(
            "PublicKeyAggregator: buffered DKG proof from party {} (buffered={})",
            msg.party_id,
            self.dkg_node_proofs.len() + 1
        );
        self.dkg_node_proofs
            .insert(msg.party_id, msg.aggregated_proof);
        self.last_ec = Some(ec.clone());

        self.try_start_cross_node_fold(&ec)
    }

    /// Start cross-node fold once we have DKG proofs from all verified honest parties.
    fn try_start_cross_node_fold(&mut self, ec: &EventContext<Sequenced>) -> Result<()> {
        let Some(honest_ids) = &self.honest_party_ids else {
            return Ok(());
        };
        let all_honest_proofs_present = honest_ids
            .iter()
            .all(|id| self.dkg_node_proofs.contains_key(id));
        if !all_honest_proofs_present || !self.cross_node_fold.is_idle() {
            return Ok(());
        }

        let mut pairs: Vec<_> = self
            .dkg_node_proofs
            .iter()
            .filter(|(pid, _)| honest_ids.contains(pid))
            .map(|(pid, p)| (*pid, p.clone()))
            .collect();
        pairs.sort_by_key(|(pid, _)| *pid);
        let proofs: Vec<Proof> = pairs.into_iter().map(|(_, p)| p).collect();

        self.cross_node_fold.start(
            proofs,
            "PublicKeyAggregator cross-node DKG fold",
            &self.bus,
            &self.e3_id,
            ec,
        )?;
        self.try_publish_complete()
    }

    /// Publish `PublicKeyAggregated` when both C5 and cross-node fold are complete.
    fn try_publish_complete(&mut self) -> Result<()> {
        let (Some(c5_proof), Some(cross_node_proof)) = (
            self.c5_proof_pending.as_ref(),
            self.cross_node_fold.result.as_ref(),
        ) else {
            return Ok(());
        };

        let PublicKeyAggregatorState::GeneratingC5Proof {
            public_key,
            public_key_hash,
            nodes,
            ..
        } = self
            .state
            .get()
            .ok_or_else(|| anyhow::anyhow!("Expected GeneratingC5Proof state"))?
        else {
            return Err(anyhow::anyhow!(
                "try_publish_complete called outside GeneratingC5Proof state"
            ));
        };

        let ec = self
            .last_ec
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No EventContext for publish"))?;

        info!("Both C5 and cross-node DKG proof ready — publishing PublicKeyAggregated");

        let event = PublicKeyAggregated {
            pubkey: public_key.clone(),
            public_key_hash,
            e3_id: self.e3_id.clone(),
            nodes: nodes.clone(),
            pk_aggregation_proof: Some(c5_proof.clone()),
            dkg_aggregated_proof: Some(cross_node_proof.clone()),
        };
        self.bus.publish(event, ec.clone())?;

        // Transition to Complete
        self.state.try_mutate(&ec, |_| {
            Ok(PublicKeyAggregatorState::Complete {
                public_key,
                keyshares: OrderedSet::new(),
                nodes,
            })
        })?;

        Ok(())
    }

    fn handle_compute_response(&mut self, msg: TypedEvent<ComputeResponse>) -> Result<()> {
        let (msg, _ec) = msg.into_components();
        if let ComputeResponseKind::Zk(ZkResponse::FoldProofs(resp)) = msg.response {
            if msg.e3_id != self.e3_id {
                return Ok(());
            }
            let ec = self
                .last_ec
                .clone()
                .ok_or_else(|| anyhow::anyhow!("No EventContext for fold response"))?;
            if self.cross_node_fold.handle_response(
                &msg.correlation_id,
                resp.proof,
                "PublicKeyAggregator cross-node DKG fold",
                &self.bus,
                &self.e3_id,
                &ec,
            )? {
                self.try_publish_complete()?;
            }
        }
        Ok(())
    }

    pub fn handle_member_expelled(
        &mut self,
        node: &str,
        ec: &EventContext<Sequenced>,
    ) -> Result<()> {
        self.state.try_mutate(ec, |mut state| {
            let PublicKeyAggregatorState::Collecting {
                threshold_n,
                threshold_m,
                keyshares,
                c1_proofs,
                nodes,
                submission_order,
                ..
            } = &mut state
            else {
                return Ok(state);
            };

            let node_str = node.to_string();

            // Find the expelled node's index in submission_order and remove from
            // all parallel collections so they stay aligned.
            if let Some(idx) = submission_order.iter().position(|(n, _)| n == &node_str) {
                let (_, expelled_keyshare) = submission_order.remove(idx);
                keyshares.remove(&expelled_keyshare);
                c1_proofs.remove(idx);
            }

            nodes.remove(&node_str);

            if *threshold_n > 0 {
                *threshold_n -= 1;
                info!(
                    "PublicKeyAggregator: reduced threshold_n to {} after expelling {}",
                    threshold_n, node
                );
            }

            if *threshold_n < *threshold_m {
                warn!(
                    "PublicKeyAggregator: threshold_n ({}) < threshold_m ({}) after expulsion — committee unviable",
                    threshold_n, threshold_m
                );
                return Ok(state);
            }

            if keyshares.len() == *threshold_n && *threshold_n > 0 {
                let m = *threshold_m;
                info!("PublicKeyAggregator: enough keyshares after expulsion, transitioning to VerifyingC1");
                return Ok(PublicKeyAggregatorState::VerifyingC1 {
                    submission_order: std::mem::take(submission_order),
                    threshold_m: m,
                    c1_proofs: std::mem::take(c1_proofs),
                    no_proof_parties: Vec::new(),
                });
            }

            Ok(state)
        })
    }
}

impl Actor for PublicKeyAggregator {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
    }
}

impl Handler<EnclaveEvent> for PublicKeyAggregator {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();
        match msg {
            EnclaveEventData::KeyshareCreated(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ShareVerificationComplete(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::PkAggregationProofSigned(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::DKGRecursiveAggregationComplete(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ComputeResponse(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ComputeRequestError(data) => {
                error!("PublicKeyAggregator received ComputeRequestError: {}", data);
            }
            EnclaveEventData::E3RequestComplete(_) => self.notify_sync(ctx, Die),
            EnclaveEventData::CommitteeMemberExpelled(data) => {
                // Only process raw events from chain (party_id not yet resolved).
                if data.party_id.is_some() {
                    return;
                }

                let node_addr = data.node.to_string();

                if data.e3_id != self.e3_id {
                    error!("Wrong e3_id sent to PublicKeyAggregator for expulsion. This should not happen.");
                    return;
                }

                info!(
                    "PublicKeyAggregator: committee member expelled: {} for e3_id={}",
                    node_addr, data.e3_id
                );
                trap(EType::PublickeyAggregation, &self.bus.with_ec(&ec), || {
                    let was_collecting = matches!(
                        self.state.get(),
                        Some(PublicKeyAggregatorState::Collecting { .. })
                    );

                    self.handle_member_expelled(&node_addr, &ec)?;

                    // If we just transitioned to VerifyingC1, dispatch C1 verification
                    // using the c1_proofs now stored in the VerifyingC1 state (already
                    // cleaned of the expelled node's entry).
                    if was_collecting {
                        if let Some(PublicKeyAggregatorState::VerifyingC1 { c1_proofs, .. }) =
                            self.state.get()
                        {
                            self.dispatch_c1_verification(&c1_proofs, ec.clone())?;
                        }
                    }
                    Ok(())
                });
            }
            _ => (),
        };
    }
}

impl Handler<TypedEvent<KeyshareCreated>> for PublicKeyAggregator {
    type Result = ();

    fn handle(
        &mut self,
        event: TypedEvent<KeyshareCreated>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let (event, ec) = event.into_components();
        trap(EType::PublickeyAggregation, &self.bus.with_ec(&ec), || {
            let e3_id = event.e3_id.clone();
            let pubkey = event.pubkey.clone();
            let node = event.node.clone();
            let c1_proof = event.signed_pk_generation_proof.clone();

            if e3_id != self.e3_id {
                error!("Wrong e3_id sent to aggregator. This should not happen.");
                return Ok(());
            }

            self.add_keyshare(pubkey, node, c1_proof, &ec)?;

            // If we just transitioned to VerifyingC1, dispatch verification
            // using c1_proofs stored in the new state.
            if let Some(PublicKeyAggregatorState::VerifyingC1 { c1_proofs, .. }) = self.state.get()
            {
                self.dispatch_c1_verification(&c1_proofs, ec)?;
            }

            Ok(())
        })
    }
}

impl Handler<TypedEvent<ShareVerificationComplete>> for PublicKeyAggregator {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ShareVerificationComplete>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::PublickeyAggregation,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_c1_verification_complete(msg),
        )
    }
}

impl Handler<TypedEvent<PkAggregationProofSigned>> for PublicKeyAggregator {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<PkAggregationProofSigned>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::PublickeyAggregation,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_pk_aggregation_proof_signed(msg),
        )
    }
}

impl Handler<TypedEvent<DKGRecursiveAggregationComplete>> for PublicKeyAggregator {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<DKGRecursiveAggregationComplete>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::PublickeyAggregation,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_dkg_recursive_aggregation_complete(msg),
        )
    }
}

impl Handler<TypedEvent<ComputeResponse>> for PublicKeyAggregator {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ComputeResponse>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::PublickeyAggregation,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_compute_response(msg),
        )
    }
}

impl Handler<Die> for PublicKeyAggregator {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}
