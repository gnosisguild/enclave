// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use anyhow::Result;
use e3_bfv_client::client::compute_pk_commitment;
use e3_data::Persistable;
use e3_events::{
    prelude::*, BusHandle, ComputeRequest, ComputeResponse, ComputeResponseKind, CorrelationId,
    Die, E3id, EnclaveEvent, EnclaveEventData, EventContext, KeyshareCreated, OrderedSet,
    PartyProofsToVerify, PkAggregationProofRequest, PkAggregationProofResponse,
    PublicKeyAggregated, Seed, Sequenced, SignedProofPayload, TypedEvent, VerifyShareProofsRequest,
    VerifyShareProofsResponse, ZkRequest, ZkResponse,
};
use e3_events::{trap, EType};
use e3_fhe::{Fhe, GetAggregatePublicKey};
use e3_fhe_params::BfvPreset;
use e3_utils::NotifySync;
use e3_utils::{ArcBytes, MAILBOX_LIMIT};
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
    },
    VerifyingC1 {
        keyshares: OrderedSet<ArcBytes>,
        nodes: OrderedSet<String>,
        threshold_m: usize,
        /// Party indices that submitted no C1 proof — treated as dishonest.
        no_proof_parties: Vec<u64>,
    },
    GeneratingC5Proof {
        public_key: Vec<u8>,
        public_key_hash: [u8; 32],
        keyshare_bytes: Vec<ArcBytes>,
        nodes: OrderedSet<String>,
    },
    Complete {
        public_key: Vec<u8>,
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
        }
    }
}

pub struct PublicKeyAggregator {
    fhe: Arc<Fhe>,
    bus: BusHandle,
    e3_id: E3id,
    state: Persistable<PublicKeyAggregatorState>,
    params_preset: BfvPreset,
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
                ..
            } = &mut state
            else {
                return Err(anyhow::anyhow!("Can only add keyshare in Collecting state"));
            };

            keyshares.insert(keyshare);
            c1_proofs.push(c1_proof);
            nodes.insert(node);
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
                    keyshares: std::mem::take(keyshares),
                    nodes: std::mem::take(nodes),
                    threshold_m: m,
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

        let request = ComputeRequest::zk(
            ZkRequest::VerifyShareProofs(VerifyShareProofsRequest { party_proofs }),
            CorrelationId::new(),
            self.e3_id.clone(),
        );
        self.bus.publish(request, ec)?;
        Ok(())
    }

    fn handle_c1_verification_response(
        &mut self,
        response: VerifyShareProofsResponse,
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
        let PublicKeyAggregatorState::VerifyingC1 {
            keyshares,
            nodes,
            threshold_m,
            no_proof_parties,
        } = self
            .state
            .get()
            .ok_or_else(|| anyhow::anyhow!("Expected VerifyingC1 state"))?
        else {
            return Err(anyhow::anyhow!(
                "handle_c1_verification_response called outside VerifyingC1 state"
            ));
        };

        // Partition honest/dishonest parties — include those that failed verification
        // AND those that submitted no C1 proof at all
        let mut dishonest_parties: Vec<u64> = no_proof_parties;
        for result in &response.party_results {
            if !result.all_verified {
                warn!(
                    "Party {} failed C1 proof verification (failed payload: {:?})",
                    result.sender_party_id, result.failed_signed_payload
                );
                dishonest_parties.push(result.sender_party_id);
            }
        }

        // Filter out dishonest parties from keyshares and nodes
        let keyshares_vec: Vec<ArcBytes> = keyshares.into_iter().collect();
        let nodes_vec: Vec<String> = nodes.into_iter().collect();

        let (honest_keyshares, honest_nodes): (Vec<ArcBytes>, Vec<String>) = keyshares_vec
            .into_iter()
            .zip(nodes_vec.into_iter())
            .enumerate()
            .filter(|(idx, _)| !dishonest_parties.contains(&(*idx as u64)))
            .map(|(_, (ks, node))| (ks, node))
            .unzip();

        if !dishonest_parties.is_empty() {
            warn!(
                "Filtered out {} dishonest parties from C1 verification: {:?}",
                dishonest_parties.len(),
                dishonest_parties
            );
        }

        // Check remaining count >= threshold
        if honest_keyshares.len() < threshold_m {
            return Err(anyhow::anyhow!(
                "Not enough honest parties after C1 verification: {} < {}",
                honest_keyshares.len(),
                threshold_m
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

        // Transition to GeneratingC5Proof
        self.state.try_mutate(&ec, |_| {
            Ok(PublicKeyAggregatorState::GeneratingC5Proof {
                public_key: pubkey.clone(),
                public_key_hash,
                keyshare_bytes: honest_keyshares.clone(),
                nodes: honest_nodes_set.clone(),
            })
        })?;

        // Dispatch C5 proof request
        let committee_h = honest_keyshares.len();

        info!("Dispatching C5 proof generation (pk aggregation)...");
        let request = ComputeRequest::zk(
            ZkRequest::PkAggregation(PkAggregationProofRequest {
                keyshare_bytes: honest_keyshares,
                aggregated_pk_bytes: ArcBytes::from_bytes(&pubkey),
                params_preset: self.params_preset.clone(),
                committee_n: committee_h, // N = all parties in this aggregation
                committee_h,
                committee_threshold: 0, // Will be resolved from preset in handler
            }),
            CorrelationId::new(),
            self.e3_id.clone(),
        );
        self.bus.publish(request, ec)?;
        Ok(())
    }

    fn handle_c5_proof_response(
        &mut self,
        response: PkAggregationProofResponse,
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
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
                "handle_c5_proof_response called outside GeneratingC5Proof state"
            ));
        };

        info!("C5 proof generated, publishing PublicKeyAggregated...");

        let proof = response.proof;

        // Transition to Complete
        self.state.try_mutate(&ec, |_| {
            Ok(PublicKeyAggregatorState::Complete {
                public_key: public_key.clone(),
                keyshares: OrderedSet::new(),
                nodes: nodes.clone(),
            })
        })?;

        // Publish PublicKeyAggregated with C5 proof
        let event = PublicKeyAggregated {
            pubkey: public_key,
            public_key_hash,
            e3_id: self.e3_id.clone(),
            nodes,
            pk_aggregation_proof: Some(proof),
        };
        self.bus.publish(event, ec)?;
        Ok(())
    }

    fn handle_compute_response(
        &mut self,
        response: ComputeResponse,
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
        match response.response {
            ComputeResponseKind::Zk(ZkResponse::VerifyShareProofs(data)) => {
                self.handle_c1_verification_response(data, ec)
            }
            ComputeResponseKind::Zk(ZkResponse::PkAggregation(data)) => {
                self.handle_c5_proof_response(data, ec)
            }
            _ => Ok(()), // Ignore other compute responses
        }
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
            EnclaveEventData::ComputeResponse(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ComputeRequestError(data) => {
                error!("PublicKeyAggregator received ComputeRequestError: {}", data);
            }
            EnclaveEventData::E3RequestComplete(_) => self.notify_sync(ctx, Die),
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

            // Extract c1_proofs before state mutation
            let c1_proofs_snapshot = match self.state.get() {
                Some(PublicKeyAggregatorState::Collecting { c1_proofs, .. }) => {
                    let mut proofs = c1_proofs.clone();
                    proofs.push(c1_proof.clone());
                    Some(proofs)
                }
                _ => None,
            };

            self.add_keyshare(pubkey, node, c1_proof, &ec)?;

            // If we just transitioned to VerifyingC1, dispatch verification
            if matches!(
                self.state.get(),
                Some(PublicKeyAggregatorState::VerifyingC1 { .. })
            ) {
                if let Some(c1_proofs) = c1_proofs_snapshot {
                    self.dispatch_c1_verification(&c1_proofs, ec)?;
                }
            }

            Ok(())
        })
    }
}

impl Handler<TypedEvent<ComputeResponse>> for PublicKeyAggregator {
    type Result = ();

    fn handle(
        &mut self,
        msg: TypedEvent<ComputeResponse>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        let (msg, ec) = msg.into_components();
        trap(EType::PublickeyAggregation, &self.bus.with_ec(&ec), || {
            self.handle_compute_response(msg, ec)
        })
    }
}

impl Handler<Die> for PublicKeyAggregator {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop();
    }
}
