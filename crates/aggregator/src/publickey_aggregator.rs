// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use anyhow::Result;
use e3_data::Persistable;
use e3_events::{
    prelude::*, BusHandle, CircuitName, ComputeRequest, ComputeResponse, ComputeResponseKind,
    CorrelationId, DKGRecursiveAggregationComplete, Die, DkgAggregationRequest, E3Failed, E3Stage,
    E3id, EnclaveEvent, EnclaveEventData, EventContext, FailureReason, KeyshareCreated, OrderedSet,
    PartyProofsToVerify, PkAggregationProofPending, PkAggregationProofRequest,
    PkAggregationProofSigned, Proof, ProofType, PublicKeyAggregated, Seed, Sequenced,
    ShareVerificationComplete, ShareVerificationDispatched, SignedProofFailed, SignedProofPayload,
    TypedEvent, VerificationKind, ZkRequest, ZkResponse,
};
use e3_events::{trap, EType};
use e3_fhe::{Fhe, GetAggregatePublicKey};
use e3_fhe_params::BfvPreset;
use e3_utils::NotifySync;
use e3_utils::{ArcBytes, MAILBOX_LIMIT};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use tracing::{error, info, warn};

/// Extract the hash-based aggregated PK commitment from the signed C5 proof.
/// This is the last public signal of `CircuitName::PkAggregation`.
fn extract_pk_commitment(c5_proof: &Proof) -> Result<[u8; 32]> {
    let layout = CircuitName::PkAggregation.output_layout();
    let bytes = layout
        .extract_field(&c5_proof.public_signals, "commitment")
        .ok_or_else(|| anyhow::anyhow!("C5 proof is missing `commitment` public signal"))?;
    let mut out = [0u8; 32];
    if bytes.len() != 32 {
        return Err(anyhow::anyhow!(
            "C5 `commitment` public signal must be 32 bytes"
        ));
    }
    out.copy_from_slice(bytes);
    Ok(out)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PublicKeyAggregatorState {
    Collecting {
        threshold_n: usize,
        threshold_m: usize,
        keyshares: OrderedSet<ArcBytes>,
        /// C1 proofs collected from KeyshareCreated events, indexed by insertion order
        /// (matches `submission_order`).
        c1_proofs: Vec<Option<SignedProofPayload>>,
        seed: Seed,
        nodes: OrderedSet<String>,
        /// Insertion-ordered (real sortition `party_id`, node, keyshare) triples.
        /// Index matches `c1_proofs`. The real `party_id` comes from `KeyshareCreated`
        /// and must be used for all downstream circuit slot indexing — arrival order
        /// is non-deterministic and does not match sortition's committee position.
        #[serde(default)]
        submission_order: Vec<(u64, String, ArcBytes)>,
    },
    VerifyingC1 {
        /// Insertion-ordered (party_id, node, keyshare) triples from Collecting.
        submission_order: Vec<(u64, String, ArcBytes)>,
        threshold_m: usize,
        /// C1 proofs in the same insertion order as `submission_order`.
        c1_proofs: Vec<Option<SignedProofPayload>>,
        /// Real party_ids that submitted no C1 proof — treated as dishonest.
        no_proof_parties: Vec<u64>,
    },
    GeneratingC5Proof {
        public_key: ArcBytes,
        keyshare_bytes: Vec<ArcBytes>,
        nodes: OrderedSet<String>,
        /// DKG recursive proofs per party (restart-critical).
        dkg_node_proofs: HashMap<u64, Option<Proof>>,
        honest_party_ids: BTreeSet<u64>,
        dishonest_parties: BTreeSet<u64>,
        /// In-flight [`ZkRequest::DkgAggregation`], if any.
        dkg_aggregation_correlation: Option<CorrelationId>,
        /// Result from [`ZkResponse::DkgAggregation`] (replaces pairwise `FoldProofs`).
        dkg_aggregated_proof: Option<Proof>,
        c5_proof_pending: Option<Proof>,
        last_ec: Option<EventContext<Sequenced>>,
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
    /// DKG recursive aggregation events received before entering GeneratingC5Proof.
    early_dkg_proofs: Vec<TypedEvent<DKGRecursiveAggregationComplete>>,
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
            early_dkg_proofs: Vec::new(),
        }
    }

    pub fn add_keyshare(
        &mut self,
        keyshare: ArcBytes,
        node: String,
        party_id: u64,
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
            info!(
                "add_keyshare: node={} party_id={} (arrival slot={})",
                node,
                party_id,
                submission_order.len()
            );
            submission_order.push((party_id, node, keyshare));
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
        submission_order: &[(u64, String, ArcBytes)],
        c1_proofs: &[Option<SignedProofPayload>],
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
        let mut party_proofs = Vec::new();
        let mut no_proof_parties = Vec::new();

        for ((party_id, _, _), proof_opt) in submission_order.iter().zip(c1_proofs.iter()) {
            match proof_opt {
                Some(proof) => {
                    party_proofs.push(PartyProofsToVerify {
                        sender_party_id: *party_id,
                        signed_proofs: vec![proof.clone()],
                    });
                }
                None => {
                    warn!(
                        "Party {} submitted keyshare without C1 proof — treating as dishonest",
                        party_id
                    );
                    no_proof_parties.push(*party_id);
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
                params_preset: self.params_preset,
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
            c1_proofs,
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

        let mut dishonest_parties = msg.dishonest_parties.clone();

        // Filter out parties that failed C1 ZK verification. Keyed by the real
        // sortition party_id carried in `submission_order`, not arrival index.
        let mut honest_entries: Vec<(u64, String, ArcBytes, Option<SignedProofPayload>)> =
            submission_order
                .into_iter()
                .zip(c1_proofs.into_iter())
                .filter(|((pid, _, _), _)| !dishonest_parties.contains(pid))
                .map(|((pid, node, ks), c1)| (pid, node, ks, c1))
                .collect();

        // Cross-check: verify each party's keyshare matches their C1 pk_commitment.
        // Parties that fail are marked dishonest and reported via SignedProofFailed.
        let mut commitment_dishonest = Vec::new();
        for (party_id, _node, ks, c1) in &honest_entries {
            let signed_proof = match c1.as_ref() {
                Some(proof) => proof,
                None => {
                    // No C1 proof for this party — should already be in dishonest_parties.
                    // If not, treat as dishonest now (defensive).
                    warn!(
                        "Party {} has no C1 proof but was not marked dishonest",
                        party_id
                    );
                    dishonest_parties.insert(*party_id);
                    continue;
                }
            };
            let ok = match e3_zk_helpers::compute_pk_commitment_from_keyshare_bytes(
                ks,
                &self.fhe.params,
                &self.fhe.crp,
            ) {
                Ok(computed) => signed_proof
                    .payload
                    .proof
                    .extract_output("pk_commitment")
                    .map_or(false, |extracted| extracted[..] == computed[..]),
                Err(e) => {
                    warn!(
                        "Failed to compute pk_commitment for party {}: {}",
                        party_id, e
                    );
                    false
                }
            };
            if !ok {
                commitment_dishonest.push((*party_id, signed_proof.clone()));
            }
        }

        // Emit SignedProofFailed for each commitment-mismatched party
        for (party_id, signed_proof) in &commitment_dishonest {
            dishonest_parties.insert(*party_id);
            match signed_proof.recover_address() {
                Ok(faulting_node) => {
                    if let Err(e) = self.bus.publish(
                        SignedProofFailed {
                            e3_id: self.e3_id.clone(),
                            faulting_node,
                            proof_type: ProofType::C1PkGeneration,
                            signed_payload: signed_proof.clone(),
                        },
                        ec.clone(),
                    ) {
                        error!("Failed to publish SignedProofFailed: {e}");
                    }
                }
                Err(e) => warn!(
                    "Could not recover address from C1 proof for party {}: {e}",
                    party_id
                ),
            }
        }

        if !commitment_dishonest.is_empty() {
            warn!(
                "C1 commitment mismatch for {} parties — filtering before aggregation",
                commitment_dishonest.len()
            );
            // Re-filter honest_entries after commitment check
            honest_entries.retain(|(pid, _, _, _)| !dishonest_parties.contains(pid));
        }

        // Sort by real party_id ascending so honest_keyshares / honest_nodes /
        // honest_party_ids all share the same ordering used by NodeFold rows
        // (publickey_aggregator sorts dkg_node_proofs by pid before dispatch)
        // and by the circuit's slot indexing in `dkg_aggregator.nr`.
        honest_entries.sort_by_key(|(pid, _, _, _)| *pid);

        let (honest_keyshares, honest_nodes): (Vec<ArcBytes>, Vec<String>) = honest_entries
            .iter()
            .map(|(_, node, ks, _)| (ks.clone(), node.clone()))
            .unzip();

        if !dishonest_parties.is_empty() {
            warn!(
                "Total dishonest parties (ZK + commitment): {:?}",
                dishonest_parties
            );
        }

        let honest_party_ids: BTreeSet<u64> =
            honest_entries.iter().map(|(pid, _, _, _)| *pid).collect();

        // Need at least threshold + 1 honest parties for aggregation
        if honest_keyshares.len() <= threshold_m {
            error!(
                "Not enough honest parties after filtering: {} (need > {})",
                honest_keyshares.len(),
                threshold_m
            );
            self.bus.publish(
                E3Failed {
                    e3_id: self.e3_id.clone(),
                    failed_at_stage: E3Stage::CommitteeFinalized,
                    reason: FailureReason::DKGInvalidShares,
                },
                ec,
            )?;
            return Ok(());
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

        let committee_h = honest_keyshares.len();
        let honest_nodes_set = OrderedSet::from(honest_nodes.clone());
        // Feed keyshares to C5 in ascending party_id order so that
        // `c5_public[i]` (pk_commitment of the i-th input keyshare) matches
        // party_ids[i] and the row-i node_fold pk bound by dkg_aggregator.nr.
        // `honest_keyshares` preserves the submission-index (== party_id) order
        // from `honest_entries`; do NOT sort by byte content.
        let keyshare_bytes: Vec<ArcBytes> = honest_keyshares.clone();

        let pubkey = ArcBytes::from_bytes(&pubkey);
        info!("Publishing PkAggregationProofPending for C5 proof generation...");
        self.bus.publish(
            PkAggregationProofPending {
                e3_id: self.e3_id.clone(),
                proof_request: PkAggregationProofRequest {
                    keyshare_bytes: keyshare_bytes.clone(),
                    aggregated_pk_bytes: pubkey.clone(),
                    params_preset: self.params_preset.clone(),
                    committee_n: committee_h,
                    committee_h,
                    committee_threshold: 0,
                },
                public_key: pubkey.clone(),
                nodes: honest_nodes_set.clone(),
            },
            ec.clone(),
        )?;

        self.state.try_mutate(&ec, |_| {
            Ok(PublicKeyAggregatorState::GeneratingC5Proof {
                public_key: pubkey.clone(),
                keyshare_bytes,
                nodes: honest_nodes_set,
                dkg_node_proofs: HashMap::new(),
                honest_party_ids: honest_party_ids.clone(),
                dishonest_parties: dishonest_parties.clone(),
                dkg_aggregation_correlation: None,
                dkg_aggregated_proof: None,
                c5_proof_pending: None,
                last_ec: Some(ec.clone()),
            })
        })?;

        // Replay any DKG proofs that arrived before we entered GeneratingC5Proof.
        let early = std::mem::take(&mut self.early_dkg_proofs);
        for event in early {
            self.handle_dkg_recursive_aggregation_complete(event)?;
        }

        self.try_dispatch_dkg_aggregation(&ec)?;

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

        let c5_proof = msg.signed_proof.payload.proof.clone();
        self.state.try_mutate(&ec, |state| {
            let PublicKeyAggregatorState::GeneratingC5Proof {
                public_key,
                keyshare_bytes,
                nodes,
                dkg_node_proofs,
                honest_party_ids,
                dishonest_parties,
                dkg_aggregation_correlation,
                dkg_aggregated_proof,
                ..
            } = state
            else {
                return Ok(state);
            };
            Ok(PublicKeyAggregatorState::GeneratingC5Proof {
                public_key,
                keyshare_bytes,
                nodes,
                dkg_node_proofs,
                honest_party_ids,
                dishonest_parties,
                dkg_aggregation_correlation,
                dkg_aggregated_proof,
                c5_proof_pending: Some(c5_proof),
                last_ec: Some(ec.clone()),
            })
        })?;
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

        let state = self.state.get();
        let Some(PublicKeyAggregatorState::GeneratingC5Proof {
            dkg_node_proofs, ..
        }) = state.as_ref()
        else {
            info!(
                "PublicKeyAggregator: early DKG proof from party {} — buffering until GeneratingC5Proof",
                msg.party_id
            );
            self.early_dkg_proofs.push(TypedEvent::new(msg, ec));
            return Ok(());
        };
        if dkg_node_proofs.contains_key(&msg.party_id) {
            warn!(
                "Duplicate DKGRecursiveAggregationComplete for party {} — ignoring",
                msg.party_id
            );
            return Ok(());
        }

        info!(
            "PublicKeyAggregator: buffered DKG proof from party {} (buffered={})",
            msg.party_id,
            dkg_node_proofs.len() + 1
        );

        self.state.try_mutate(&ec, |state| {
            let PublicKeyAggregatorState::GeneratingC5Proof {
                public_key,
                keyshare_bytes,
                nodes,
                mut dkg_node_proofs,
                honest_party_ids,
                dishonest_parties,
                dkg_aggregation_correlation,
                dkg_aggregated_proof,
                c5_proof_pending,
                last_ec: _,
            } = state
            else {
                return Ok(state);
            };
            dkg_node_proofs.insert(msg.party_id, msg.aggregated_proof);
            Ok(PublicKeyAggregatorState::GeneratingC5Proof {
                public_key,
                keyshare_bytes,
                nodes,
                dkg_node_proofs,
                honest_party_ids,
                dishonest_parties,
                dkg_aggregation_correlation,
                dkg_aggregated_proof,
                c5_proof_pending,
                last_ec: Some(ec.clone()),
            })
        })?;

        self.try_dispatch_dkg_aggregation(&ec)
    }

    /// Dispatch [`ZkRequest::DkgAggregation`] once C5 and all honest NodeFold proofs are ready.
    fn try_dispatch_dkg_aggregation(&mut self, ec: &EventContext<Sequenced>) -> Result<()> {
        let state = self.state.get();
        let Some(PublicKeyAggregatorState::GeneratingC5Proof {
            dkg_node_proofs,
            honest_party_ids,
            c5_proof_pending,
            dkg_aggregation_correlation,
            dkg_aggregated_proof,
            ..
        }) = state.as_ref()
        else {
            return Ok(());
        };

        let Some(c5_proof) = c5_proof_pending.as_ref() else {
            return Ok(());
        };

        if dkg_aggregation_correlation.is_some() || dkg_aggregated_proof.is_some() {
            return Ok(());
        }

        let all_honest_proofs_present = honest_party_ids
            .iter()
            .all(|id| dkg_node_proofs.contains_key(id));
        if !all_honest_proofs_present {
            return Ok(());
        }

        let mut pairs: Vec<_> = dkg_node_proofs
            .iter()
            .filter(|(pid, _)| honest_party_ids.contains(pid))
            .filter_map(|(pid, p)| p.as_ref().map(|proof| (*pid, proof.clone())))
            .collect();
        pairs.sort_by_key(|(pid, _)| *pid);
        let party_ids: Vec<u64> = pairs.iter().map(|(pid, _)| *pid).collect();
        let node_fold_proofs: Vec<Proof> = pairs.into_iter().map(|(_, p)| p).collect();
        info!(
            "ORDER-DEBUG dispatch DkgAggregation: honest_party_ids(submission-idx)={:?} \
             dkg_node_proofs_keys(real party_id from DKGRecursiveAggregationComplete)={:?} \
             party_ids_passed_to_circuit={:?}",
            honest_party_ids.iter().collect::<Vec<_>>(),
            {
                let mut k: Vec<u64> = dkg_node_proofs.keys().copied().collect();
                k.sort();
                k
            },
            party_ids
        );

        if node_fold_proofs.is_empty() {
            info!("PublicKeyAggregator: proof aggregation disabled — skipping DkgAggregation");
            self.try_publish_complete()?;
            return Ok(());
        }

        let corr = CorrelationId::new();
        self.bus.publish(
            ComputeRequest::zk(
                ZkRequest::DkgAggregation(DkgAggregationRequest {
                    node_fold_proofs,
                    c5_proof: c5_proof.clone(),
                    party_ids,
                    params_preset: self.params_preset,
                }),
                corr,
                self.e3_id.clone(),
            ),
            ec.clone(),
        )?;

        self.state.try_mutate(ec, |state| {
            let PublicKeyAggregatorState::GeneratingC5Proof {
                public_key,
                keyshare_bytes,
                nodes,
                dkg_node_proofs,
                honest_party_ids,
                dishonest_parties,
                dkg_aggregation_correlation: _,
                dkg_aggregated_proof,
                c5_proof_pending,
                last_ec,
            } = state
            else {
                return Ok(state);
            };
            Ok(PublicKeyAggregatorState::GeneratingC5Proof {
                public_key,
                keyshare_bytes,
                nodes,
                dkg_node_proofs,
                honest_party_ids,
                dishonest_parties,
                dkg_aggregation_correlation: Some(corr),
                dkg_aggregated_proof,
                c5_proof_pending,
                last_ec,
            })
        })?;
        Ok(())
    }

    /// Publish `PublicKeyAggregated` when C5 (non-ZK recursive) and, if applicable, the EVM DkgAggregator proof are ready (or aggregation skipped).
    fn try_publish_complete(&mut self) -> Result<()> {
        if let Some(ec) = self.state.get().and_then(|s| {
            if let PublicKeyAggregatorState::GeneratingC5Proof { last_ec, .. } = &s {
                last_ec.clone()
            } else {
                None
            }
        }) {
            self.try_dispatch_dkg_aggregation(&ec)?;
        }

        let PublicKeyAggregatorState::GeneratingC5Proof {
            public_key,
            nodes,
            c5_proof_pending,
            dkg_aggregated_proof,
            dkg_aggregation_correlation: _,
            last_ec,
            ..
        } = self
            .state
            .get()
            .ok_or_else(|| anyhow::anyhow!("Expected GeneratingC5Proof state"))?
        else {
            return Ok(());
        };

        let Some(c5_proof) = c5_proof_pending.as_ref() else {
            return Ok(());
        };

        let all_proofs_are_none = self
            .state
            .get()
            .and_then(|s| {
                if let PublicKeyAggregatorState::GeneratingC5Proof {
                    dkg_node_proofs,
                    honest_party_ids,
                    ..
                } = &s
                {
                    let all_present = honest_party_ids
                        .iter()
                        .all(|id| dkg_node_proofs.contains_key(id));
                    Some(all_present && dkg_node_proofs.values().all(|p| p.is_none()))
                } else {
                    None
                }
            })
            .unwrap_or(false);

        if !all_proofs_are_none && dkg_aggregated_proof.is_none() {
            return Ok(());
        }

        let ec = last_ec
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No EventContext for publish"))?;

        info!(
            "Publishing PublicKeyAggregated (dkg_evm_proof={})",
            if dkg_aggregated_proof.is_some() {
                "present"
            } else {
                "skipped"
            }
        );

        let pk_commitment = extract_pk_commitment(c5_proof)?;

        let event = PublicKeyAggregated {
            pubkey: public_key.clone(),
            e3_id: self.e3_id.clone(),
            nodes: nodes.clone(),
            pk_commitment,
            dkg_aggregator_proof: dkg_aggregated_proof.clone(),
        };
        self.bus.publish(event, ec.clone())?;

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
        if let ComputeResponseKind::Zk(ZkResponse::DkgAggregation(resp)) = msg.response {
            if msg.e3_id != self.e3_id {
                return Ok(());
            }
            let state = self.state.get();
            let Some(PublicKeyAggregatorState::GeneratingC5Proof { last_ec, .. }) = state.as_ref()
            else {
                return Ok(());
            };
            let Some(_ec) = last_ec.clone() else {
                return Err(anyhow::anyhow!(
                    "No EventContext for DkgAggregation response"
                ));
            };
            self.state.try_mutate_without_context(|state| {
                let PublicKeyAggregatorState::GeneratingC5Proof {
                    public_key,
                    keyshare_bytes,
                    nodes,
                    dkg_node_proofs,
                    honest_party_ids,
                    dishonest_parties,
                    dkg_aggregation_correlation,
                    dkg_aggregated_proof,
                    c5_proof_pending,
                    last_ec,
                } = state
                else {
                    return Ok(state);
                };
                if dkg_aggregation_correlation.as_ref() != Some(&msg.correlation_id) {
                    return Ok(PublicKeyAggregatorState::GeneratingC5Proof {
                        public_key,
                        keyshare_bytes,
                        nodes,
                        dkg_node_proofs,
                        honest_party_ids,
                        dishonest_parties,
                        dkg_aggregation_correlation,
                        dkg_aggregated_proof,
                        c5_proof_pending,
                        last_ec,
                    });
                }
                Ok(PublicKeyAggregatorState::GeneratingC5Proof {
                    public_key,
                    keyshare_bytes,
                    nodes,
                    dkg_node_proofs,
                    honest_party_ids,
                    dishonest_parties,
                    dkg_aggregation_correlation: None,
                    dkg_aggregated_proof: Some(resp.proof.clone()),
                    c5_proof_pending,
                    last_ec,
                })
            })?;
            self.try_publish_complete()?;
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
            if let Some(idx) = submission_order.iter().position(|(_, n, _)| n == &node_str) {
                let (_, _, expelled_keyshare) = submission_order.remove(idx);
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
                        if let Some(PublicKeyAggregatorState::VerifyingC1 {
                            submission_order,
                            c1_proofs,
                            ..
                        }) = self.state.get()
                        {
                            self.dispatch_c1_verification(
                                &submission_order,
                                &c1_proofs,
                                ec.clone(),
                            )?;
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
            let party_id = event.party_id;
            let c1_proof = event.signed_pk_generation_proof.clone();

            if e3_id != self.e3_id {
                error!("Wrong e3_id sent to aggregator. This should not happen.");
                return Ok(());
            }

            self.add_keyshare(pubkey, node, party_id, c1_proof, &ec)?;

            // If we just transitioned to VerifyingC1, dispatch verification
            // using c1_proofs stored in the new state.
            if let Some(PublicKeyAggregatorState::VerifyingC1 {
                submission_order,
                c1_proofs,
                ..
            }) = self.state.get()
            {
                self.dispatch_c1_verification(&submission_order, &c1_proofs, ec)?;
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
