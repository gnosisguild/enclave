// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use crate::proof_fold::ProofFoldState;
use actix::prelude::*;
use anyhow::Result;
use e3_data::Persistable;
use e3_events::{
    prelude::*, BusHandle, ComputeRequestErrorKind, ComputeResponse, ComputeResponseKind,
    DKGRecursiveAggregationComplete, Die, E3id, EnclaveEvent, EnclaveEventData, EventContext,
    KeyshareCreated, OrderedSet, PartyProofsToVerify, PkAggregationProofPending,
    PkAggregationProofRequest, PkAggregationProofSigned, Proof, ProofType, PublicKeyAggregated,
    Seed, Sequenced, ShareVerificationComplete, ShareVerificationDispatched, SignedProofFailed,
    SignedProofPayload, TypedEvent, VerificationKind, ZkError, ZkResponse,
};
use e3_events::{trap, EType};
use e3_fhe::{Fhe, GetAggregatePublicKey};
use e3_fhe_params::BfvPreset;
use e3_utils::NotifySync;
use e3_utils::{ArcBytes, MAILBOX_LIMIT};
use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use tracing::{error, info, warn};

/// Derive c1_commitments from signed proofs by extracting pk_commitment from each.
fn derive_c1_commitments(signed_proofs: &[Option<SignedProofPayload>]) -> Vec<ArcBytes> {
    signed_proofs
        .iter()
        .enumerate()
        .filter_map(|(i, opt)| {
            let sp = opt.as_ref()?;
            let proof = &sp.payload.proof;
            tracing::info!(
                "C1 proof[{}]: circuit={:?}, public_signals_len={}, signals_hex={}",
                i,
                proof.circuit,
                proof.public_signals.len(),
                proof.public_signals[..std::cmp::min(128, proof.public_signals.len())]
                    .iter()
                    .map(|b| format!("{:02x}", b))
                    .collect::<String>()
            );
            let commitment = proof.extract_output("pk_commitment");
            if let Some(ref c) = commitment {
                tracing::info!(
                    "C1 proof[{}]: extracted pk_commitment={}",
                    i,
                    c.iter().map(|b| format!("{:02x}", b)).collect::<String>()
                );
            } else {
                tracing::warn!("C1 proof[{}]: failed to extract pk_commitment", i);
            }
            commitment
        })
        .collect()
}

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
        keyshare_bytes: Vec<ArcBytes>,
        /// Signed C1 proofs from honest parties, aligned with `keyshare_bytes`.
        /// Commitments are extracted on the fly via `extract_output("pk_commitment")`.
        /// Retained for fault attribution if a commitment mismatch is detected later.
        c1_signed_proofs: Vec<Option<SignedProofPayload>>,
        nodes: OrderedSet<String>,
        /// DKG recursive proofs per party (restart-critical).
        dkg_node_proofs: HashMap<u64, Option<Proof>>,
        honest_party_ids: BTreeSet<u64>,
        dishonest_parties: BTreeSet<u64>,
        cross_node_fold: ProofFoldState,
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

        let dishonest_parties = &msg.dishonest_parties;
        let total_parties = submission_order.len();

        // Filter out dishonest parties using submission_order (insertion-order indexed,
        // matching the party IDs sent to dispatch_c1_verification).
        let honest_entries: Vec<(usize, (String, ArcBytes))> = submission_order
            .into_iter()
            .enumerate()
            .filter(|(idx, _)| !dishonest_parties.contains(&(*idx as u64)))
            .collect();

        let (honest_keyshares, honest_nodes): (Vec<ArcBytes>, Vec<String>) = honest_entries
            .iter()
            .map(|(_, (node, ks))| (ks.clone(), node.clone()))
            .unzip();

        // Collect signed C1 proofs from honest parties (commitments derived on the fly)
        let c1_signed_proofs: Vec<Option<SignedProofPayload>> = honest_entries
            .iter()
            .map(|(idx, _)| c1_proofs.get(*idx).and_then(|opt| opt.clone()))
            .collect();

        if !dishonest_parties.is_empty() {
            warn!(
                "Filtered out {} dishonest parties from C1 verification: {:?}",
                dishonest_parties.len(),
                dishonest_parties
            );
        }

        let honest_party_ids: BTreeSet<u64> = (0..total_parties as u64)
            .filter(|id| !dishonest_parties.contains(id))
            .collect();

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

        let committee_h = honest_keyshares.len();
        let honest_nodes_set = OrderedSet::from(honest_nodes);
        // keyshare_bytes follows OrderedSet ordering (used by C5 prover).
        // Re-align c1_signed_proofs to the same order so c1_commitments[i]
        // corresponds to keyshare_bytes[i].
        let keyshare_bytes: Vec<_> = honest_keyshares_set.iter().cloned().collect();
        let c1_signed_proofs = {
            // Build a map from keyshare → c1 proof, then iterate in OrderedSet order.
            let ks_to_proof: std::collections::HashMap<Vec<u8>, &Option<SignedProofPayload>> =
                honest_keyshares
                    .iter()
                    .zip(c1_signed_proofs.iter())
                    .map(|(ks, proof)| (ks.to_vec(), proof))
                    .collect();
            keyshare_bytes
                .iter()
                .map(|ks| ks_to_proof.get(&ks.to_vec()).and_then(|opt| (*opt).clone()))
                .collect::<Vec<_>>()
        };

        // Publish pending event before transitioning state so a publish
        // failure leaves us in VerifyingC1 (retryable) rather than
        // GeneratingC5Proof (no retry path).
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
                    c1_commitments: derive_c1_commitments(&c1_signed_proofs),
                },
                public_key: pubkey.clone(),
                nodes: honest_nodes_set.clone(),
            },
            ec.clone(),
        )?;

        self.state.try_mutate(&ec, |_| {
            Ok(PublicKeyAggregatorState::GeneratingC5Proof {
                public_key: pubkey.clone(),
                keyshare_bytes: honest_keyshares,
                c1_signed_proofs,
                nodes: honest_nodes_set,
                dkg_node_proofs: HashMap::new(),
                honest_party_ids: honest_party_ids.clone(),
                dishonest_parties: dishonest_parties.clone(),
                cross_node_fold: ProofFoldState::new(),
                c5_proof_pending: None,
                last_ec: Some(ec.clone()),
            })
        })?;

        // Replay any DKG proofs that arrived before we entered GeneratingC5Proof.
        let early = std::mem::take(&mut self.early_dkg_proofs);
        for event in early {
            self.handle_dkg_recursive_aggregation_complete(event)?;
        }

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

        let c5_proof = msg.signed_proof.payload.proof.clone();
        self.state.try_mutate(&ec, |state| {
            let PublicKeyAggregatorState::GeneratingC5Proof {
                public_key,
                keyshare_bytes,
                c1_signed_proofs,
                nodes,
                dkg_node_proofs,
                honest_party_ids,
                dishonest_parties,
                cross_node_fold,
                ..
            } = state
            else {
                return Ok(state);
            };
            Ok(PublicKeyAggregatorState::GeneratingC5Proof {
                public_key,
                keyshare_bytes,
                c1_signed_proofs,
                nodes,
                dkg_node_proofs,
                honest_party_ids,
                dishonest_parties,
                cross_node_fold,
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
                c1_signed_proofs,
                nodes,
                mut dkg_node_proofs,
                honest_party_ids,
                dishonest_parties,
                cross_node_fold,
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
                c1_signed_proofs,
                nodes,
                dkg_node_proofs,
                honest_party_ids,
                dishonest_parties,
                cross_node_fold,
                c5_proof_pending,
                last_ec: Some(ec.clone()),
            })
        })?;

        self.try_start_cross_node_fold(&ec)
    }

    /// Start cross-node fold once we have DKG proofs from all verified honest parties.
    fn try_start_cross_node_fold(&mut self, ec: &EventContext<Sequenced>) -> Result<()> {
        let state = self.state.get();
        let Some(PublicKeyAggregatorState::GeneratingC5Proof {
            dkg_node_proofs,
            honest_party_ids,
            cross_node_fold,
            ..
        }) = state.as_ref()
        else {
            return Ok(());
        };
        let all_honest_proofs_present = honest_party_ids
            .iter()
            .all(|id| dkg_node_proofs.contains_key(id));
        if !all_honest_proofs_present
            || (!cross_node_fold.is_idle() && !cross_node_fold.needs_restart())
        {
            return Ok(());
        }

        // Collect non-None proofs from honest parties for cross-node folding.
        // Folding is skipped only when all honest-party proofs are None (every node
        // reported aggregation disabled). A mixed Some/None scenario should not occur
        // in practice because proof_aggregation_enabled is an E3-level flag shared by all nodes.
        let mut pairs: Vec<_> = dkg_node_proofs
            .iter()
            .filter(|(pid, _)| honest_party_ids.contains(pid))
            .filter_map(|(pid, p)| p.as_ref().map(|proof| (*pid, proof.clone())))
            .collect();
        pairs.sort_by_key(|(pid, _)| *pid);
        let proofs: Vec<Proof> = pairs.into_iter().map(|(_, p)| p).collect();

        // If no proofs to fold (aggregation was disabled), try publishing immediately
        if proofs.is_empty() {
            info!("PublicKeyAggregator: proof aggregation disabled — skipping cross-node fold");
            self.try_publish_complete()?;
            return Ok(());
        }

        self.state.try_mutate(ec, |state| {
            let PublicKeyAggregatorState::GeneratingC5Proof {
                public_key,
                keyshare_bytes,
                c1_signed_proofs,
                nodes,
                dkg_node_proofs,
                honest_party_ids,
                dishonest_parties,
                mut cross_node_fold,
                c5_proof_pending,
                last_ec,
            } = state
            else {
                return Ok(state);
            };
            if cross_node_fold.needs_restart() {
                warn!("cross-node fold stuck mid-step on restart — resetting and re-folding from persisted proofs");
                cross_node_fold = ProofFoldState::new();
            }
            cross_node_fold.start(
                proofs,
                "PublicKeyAggregator cross-node DKG fold",
                &self.bus,
                &self.e3_id,
                ec,
            )?;
            Ok(PublicKeyAggregatorState::GeneratingC5Proof {
                public_key,
                keyshare_bytes,
                c1_signed_proofs,
                nodes,
                dkg_node_proofs,
                honest_party_ids,
                dishonest_parties,
                cross_node_fold,
                c5_proof_pending,
                last_ec,
            })
        })?;
        self.try_publish_complete()
    }

    /// Publish `PublicKeyAggregated` when both C5 and cross-node fold are complete.
    fn try_publish_complete(&mut self) -> Result<()> {
        let PublicKeyAggregatorState::GeneratingC5Proof {
            public_key,
            nodes,
            c5_proof_pending,
            cross_node_fold,
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

        // Cross-node fold result is optional — None when proof aggregation is disabled
        let dkg_aggregated_proof = cross_node_fold.result.clone();

        // If aggregation is enabled but fold hasn't completed yet, wait
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

        if dkg_aggregated_proof.is_none() && !all_proofs_are_none {
            // Aggregation is enabled but fold not done yet — wait
            return Ok(());
        }

        let ec = last_ec
            .clone()
            .ok_or_else(|| anyhow::anyhow!("No EventContext for publish"))?;

        info!(
            "C5 proof ready — publishing PublicKeyAggregated (dkg_aggregated_proof={})",
            if dkg_aggregated_proof.is_some() {
                "present"
            } else {
                "skipped"
            }
        );

        let event = PublicKeyAggregated {
            pubkey: public_key.clone(),
            e3_id: self.e3_id.clone(),
            nodes: nodes.clone(),
            pk_aggregation_proof: Some(c5_proof.clone()),
            dkg_aggregated_proof,
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
            let state = self.state.get();
            let Some(PublicKeyAggregatorState::GeneratingC5Proof { last_ec, .. }) = state.as_ref()
            else {
                // Late response after transitioning out of GeneratingC5Proof — ignore.
                return Ok(());
            };
            let Some(ec) = last_ec.clone() else {
                return Err(anyhow::anyhow!("No EventContext for fold response"));
            };
            self.state.try_mutate_without_context(|state| {
                let PublicKeyAggregatorState::GeneratingC5Proof {
                    public_key,
                    keyshare_bytes,
                    c1_signed_proofs,
                    nodes,
                    dkg_node_proofs,
                    honest_party_ids,
                    dishonest_parties,
                    mut cross_node_fold,
                    c5_proof_pending,
                    last_ec,
                } = state
                else {
                    return Ok(state);
                };
                cross_node_fold.handle_response(
                    &msg.correlation_id,
                    resp.proof.clone(),
                    "PublicKeyAggregator cross-node DKG fold",
                    &self.bus,
                    &self.e3_id,
                    &ec,
                )?;
                Ok(PublicKeyAggregatorState::GeneratingC5Proof {
                    public_key,
                    keyshare_bytes,
                    c1_signed_proofs,
                    nodes,
                    dkg_node_proofs,
                    honest_party_ids,
                    dishonest_parties,
                    cross_node_fold,
                    c5_proof_pending,
                    last_ec,
                })
            })?;
            self.try_publish_complete()?;
        }
        Ok(())
    }

    fn handle_c1_commitment_mismatch(
        &mut self,
        mismatched_indices: &[usize],
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
        let PublicKeyAggregatorState::GeneratingC5Proof {
            keyshare_bytes,
            c1_signed_proofs: stored_c1_signed_proofs,
            nodes,
            dkg_node_proofs,
            honest_party_ids,
            dishonest_parties,
            ..
        } = self
            .state
            .get()
            .ok_or_else(|| anyhow::anyhow!("Expected GeneratingC5Proof state"))?
        else {
            return Err(anyhow::anyhow!(
                "handle_c1_commitment_mismatch called outside GeneratingC5Proof state"
            ));
        };

        // Map keyshare-order indices to original party IDs.
        // keyshare_bytes[i] corresponds to the i-th element in honest_party_ids (sorted).
        let honest_ids_sorted: Vec<u64> = honest_party_ids.iter().copied().collect();
        let mut newly_dishonest: BTreeSet<u64> = BTreeSet::new();
        for &idx in mismatched_indices {
            if let Some(&party_id) = honest_ids_sorted.get(idx) {
                warn!(
                    "C1 commitment mismatch for party {} (index {}) — marking as dishonest",
                    party_id, idx
                );
                newly_dishonest.insert(party_id);
            } else {
                warn!(
                    "C1 commitment mismatch index {} out of range (honest parties: {})",
                    idx,
                    honest_ids_sorted.len()
                );
            }
        }

        if newly_dishonest.is_empty() {
            return Err(anyhow::anyhow!(
                "C1 commitment mismatch reported but no valid party indices"
            ));
        }

        // Emit SignedProofFailed for each mismatched party that has a signed C1 proof
        for &idx in mismatched_indices {
            if let Some(Some(signed_payload)) = stored_c1_signed_proofs.get(idx) {
                match signed_payload.recover_address() {
                    Ok(faulting_node) => {
                        if let Err(err) = self.bus.publish(
                            SignedProofFailed {
                                e3_id: self.e3_id.clone(),
                                faulting_node,
                                proof_type: ProofType::C1PkGeneration,
                                signed_payload: signed_payload.clone(),
                            },
                            ec.clone(),
                        ) {
                            error!("Failed to publish SignedProofFailed for C1 mismatch at index {}: {err}", idx);
                        }
                    }
                    Err(err) => {
                        warn!(
                            "Could not recover address from C1 signed proof at index {}: {err}",
                            idx
                        );
                    }
                }
            }
        }

        // Filter out the newly dishonest parties
        let remaining_keyshares: Vec<ArcBytes> = keyshare_bytes
            .iter()
            .enumerate()
            .filter(|(i, _)| !mismatched_indices.contains(i))
            .map(|(_, ks)| ks.clone())
            .collect();

        let remaining_ids: BTreeSet<u64> = honest_party_ids
            .iter()
            .copied()
            .filter(|id| !newly_dishonest.contains(id))
            .collect();

        let remaining_nodes: OrderedSet<String> = {
            let nodes_vec: Vec<String> = nodes
                .iter()
                .enumerate()
                .filter(|(i, _)| !mismatched_indices.contains(i))
                .map(|(_, n)| n.clone())
                .collect();
            OrderedSet::from(nodes_vec)
        };

        let mut all_dishonest = dishonest_parties.clone();
        all_dishonest.extend(newly_dishonest.iter());

        // Check if enough honest parties remain
        let remaining_count = remaining_keyshares.len();
        // We need > 0 honest parties; the circuit enforces the threshold check
        if remaining_count == 0 {
            return Err(anyhow::anyhow!(
                "No honest parties remaining after C1 commitment mismatch filtering"
            ));
        }

        info!(
            "Re-aggregating public key from {} remaining honest parties (removed {} dishonest)",
            remaining_count,
            newly_dishonest.len()
        );

        // Re-aggregate the public key without the dishonest parties
        let remaining_keyshares_set = OrderedSet::from(remaining_keyshares.clone());
        let pubkey = self.fhe.get_aggregate_public_key(GetAggregatePublicKey {
            keyshares: remaining_keyshares_set,
        })?;

        let committee_h = remaining_count;
        let pubkey = ArcBytes::from_bytes(&pubkey);

        // Filter c1_signed_proofs to match remaining honest parties
        let remaining_c1_signed_proofs: Vec<Option<SignedProofPayload>> = stored_c1_signed_proofs
            .iter()
            .enumerate()
            .filter(|(i, _)| !mismatched_indices.contains(i))
            .map(|(_, sp)| sp.clone())
            .collect();

        // Publish new PkAggregationProofPending
        self.bus.publish(
            PkAggregationProofPending {
                e3_id: self.e3_id.clone(),
                proof_request: PkAggregationProofRequest {
                    keyshare_bytes: remaining_keyshares.clone(),
                    aggregated_pk_bytes: pubkey.clone(),
                    params_preset: self.params_preset.clone(),
                    committee_n: committee_h,
                    committee_h,
                    committee_threshold: 0,
                    c1_commitments: derive_c1_commitments(&remaining_c1_signed_proofs),
                },
                public_key: pubkey.clone(),
                nodes: remaining_nodes.clone(),
            },
            ec.clone(),
        )?;

        // Keep DKG proofs from remaining honest parties — they won't be re-delivered.
        let remaining_dkg_proofs: HashMap<u64, Option<Proof>> = dkg_node_proofs
            .into_iter()
            .filter(|(pid, _)| remaining_ids.contains(pid))
            .collect();

        // Transition state: reset fold but preserve honest DKG proofs
        self.state.try_mutate(&ec, |_| {
            Ok(PublicKeyAggregatorState::GeneratingC5Proof {
                public_key: pubkey.clone(),
                keyshare_bytes: remaining_keyshares,
                c1_signed_proofs: remaining_c1_signed_proofs,
                nodes: remaining_nodes,
                dkg_node_proofs: remaining_dkg_proofs,
                honest_party_ids: remaining_ids,
                dishonest_parties: all_dishonest,
                cross_node_fold: ProofFoldState::new(),
                c5_proof_pending: None,
                last_ec: Some(ec.clone()),
            })
        })?;

        self.try_start_cross_node_fold(&ec)?;

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
                if data.request().e3_id != self.e3_id {
                    return;
                }
                if let ComputeRequestErrorKind::Zk(ZkError::C1CommitmentMismatch {
                    ref mismatched_indices,
                }) = data.get_err()
                {
                    let indices = mismatched_indices.clone();
                    trap(EType::PublickeyAggregation, &self.bus.with_ec(&ec), || {
                        self.handle_c1_commitment_mismatch(&indices, ec.clone())
                    });
                } else {
                    error!("PublicKeyAggregator received ComputeRequestError: {}", data);
                }
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
