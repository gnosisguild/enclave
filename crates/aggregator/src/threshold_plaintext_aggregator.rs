// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::collections::{BTreeSet, HashMap};

use crate::proof_fold::ProofFoldState;
use actix::prelude::*;
use anyhow::{anyhow, bail, ensure, Result};
use e3_data::Persistable;
use e3_events::{
    prelude::*, trap, AggregationProofPending, AggregationProofSigned, BusHandle, ComputeRequest,
    ComputeResponse, ComputeResponseKind, CorrelationId, DecryptedSharesAggregationProofRequest,
    DecryptionshareCreated, Die, E3id, EType, EnclaveEvent, EnclaveEventData, EventContext,
    PartyProofsToVerify, PlaintextAggregated, Proof, Seed, Sequenced, ShareVerificationComplete,
    ShareVerificationDispatched, SignedProofPayload, TypedEvent, VerificationKind, ZkResponse,
};
use e3_fhe_params::BfvPreset;
use e3_sortition::{E3CommitteeContainsRequest, E3CommitteeContainsResponse, Sortition};
use e3_trbfv::{
    calculate_threshold_decryption::CalculateThresholdDecryptionRequest, TrBFVConfig, TrBFVRequest,
    TrBFVResponse,
};
use e3_utils::NotifySync;
use e3_utils::{utility_types::ArcBytes, MAILBOX_LIMIT};
use e3_zk_helpers::circuits::threshold::decrypted_shares_aggregation::MAX_MSG_NON_ZERO_COEFFS;
use tracing::{debug, info, trace, warn};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Collecting {
    threshold_m: u64,
    threshold_n: u64,
    shares: HashMap<u64, Vec<ArcBytes>>,
    /// Signed raw C6 proofs for ShareVerification.
    c6_proofs: HashMap<u64, Vec<SignedProofPayload>>,
    /// Wrapped C6 proofs for cross-node fold.
    #[serde(default)]
    c6_wrapped_proofs: HashMap<u64, Vec<Proof>>,
    seed: Seed,
    ciphertext_output: Vec<ArcBytes>,
    params: ArcBytes,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerifyingC6 {
    threshold_m: u64,
    threshold_n: u64,
    shares: HashMap<u64, Vec<ArcBytes>>,
    c6_proofs: HashMap<u64, Vec<SignedProofPayload>>,
    #[serde(default)]
    c6_wrapped_proofs: HashMap<u64, Vec<Proof>>,
    ciphertext_output: Vec<ArcBytes>,
    params: ArcBytes,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Computing {
    threshold_m: u64,
    threshold_n: u64,
    shares: Vec<(u64, Vec<ArcBytes>)>,
    ciphertext_output: Vec<ArcBytes>,
    params: ArcBytes,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct GeneratingC7Proof {
    threshold_m: u64,
    threshold_n: u64,
    shares: Vec<(u64, Vec<ArcBytes>)>,
    plaintext: Vec<ArcBytes>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Complete {
    decrypted: Vec<ArcBytes>,
    shares: Vec<(u64, Vec<ArcBytes>)>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ThresholdPlaintextAggregatorState {
    Collecting(Collecting),
    VerifyingC6(VerifyingC6),
    Computing(Computing),
    GeneratingC7Proof(GeneratingC7Proof),
    Complete(Complete),
}

impl TryFrom<ThresholdPlaintextAggregatorState> for Collecting {
    type Error = anyhow::Error;
    fn try_from(
        value: ThresholdPlaintextAggregatorState,
    ) -> std::result::Result<Self, Self::Error> {
        match value {
            ThresholdPlaintextAggregatorState::Collecting(s) => Ok(s),
            _ => bail!("PlaintextState was expected to be Collecting but it was not."),
        }
    }
}

impl TryFrom<ThresholdPlaintextAggregatorState> for VerifyingC6 {
    type Error = anyhow::Error;
    fn try_from(
        value: ThresholdPlaintextAggregatorState,
    ) -> std::result::Result<Self, Self::Error> {
        match value {
            ThresholdPlaintextAggregatorState::VerifyingC6(s) => Ok(s),
            _ => bail!("Inconsistent state: expected VerifyingC6"),
        }
    }
}

impl TryFrom<ThresholdPlaintextAggregatorState> for Computing {
    type Error = anyhow::Error;
    fn try_from(
        value: ThresholdPlaintextAggregatorState,
    ) -> std::result::Result<Self, Self::Error> {
        match value {
            ThresholdPlaintextAggregatorState::Computing(s) => Ok(s),
            _ => bail!("Inconsistent state: expected Computing"),
        }
    }
}

impl TryFrom<ThresholdPlaintextAggregatorState> for GeneratingC7Proof {
    type Error = anyhow::Error;
    fn try_from(
        value: ThresholdPlaintextAggregatorState,
    ) -> std::result::Result<Self, Self::Error> {
        match value {
            ThresholdPlaintextAggregatorState::GeneratingC7Proof(s) => Ok(s),
            _ => bail!("Inconsistent state: expected GeneratingC7Proof"),
        }
    }
}

impl TryFrom<ThresholdPlaintextAggregatorState> for Complete {
    type Error = anyhow::Error;
    fn try_from(
        value: ThresholdPlaintextAggregatorState,
    ) -> std::result::Result<Self, Self::Error> {
        match value {
            ThresholdPlaintextAggregatorState::Complete(s) => Ok(s),
            _ => bail!("Inconsistent state: expected Complete"),
        }
    }
}

impl ThresholdPlaintextAggregatorState {
    pub fn init(
        threshold_m: u64,
        threshold_n: u64,
        seed: Seed,
        ciphertext_output: Vec<ArcBytes>,
        params: ArcBytes,
    ) -> Self {
        ThresholdPlaintextAggregatorState::Collecting(Collecting {
            threshold_m,
            threshold_n,
            shares: HashMap::new(),
            c6_proofs: HashMap::new(),
            c6_wrapped_proofs: HashMap::new(),
            seed,
            ciphertext_output,
            params,
        })
    }
}

pub struct ThresholdPlaintextAggregator {
    bus: BusHandle,
    sortition: Addr<Sortition>,
    e3_id: E3id,
    params_preset: BfvPreset,
    state: Persistable<ThresholdPlaintextAggregatorState>,
    /// C6 cross-node proof fold state.
    c6_fold: ProofFoldState,
    /// C7 proofs stored while waiting for C6 fold completion.
    c7_proofs_pending: Option<Vec<Proof>>,
    /// Last event context, reused for fold steps and final publish.
    last_ec: Option<EventContext<Sequenced>>,
}

pub struct ThresholdPlaintextAggregatorParams {
    pub bus: BusHandle,
    pub sortition: Addr<Sortition>,
    pub e3_id: E3id,
    pub params_preset: BfvPreset,
}

impl ThresholdPlaintextAggregator {
    pub fn new(
        params: ThresholdPlaintextAggregatorParams,
        state: Persistable<ThresholdPlaintextAggregatorState>,
    ) -> Self {
        ThresholdPlaintextAggregator {
            bus: params.bus,
            sortition: params.sortition,
            e3_id: params.e3_id,
            params_preset: params.params_preset,
            state,
            c6_fold: ProofFoldState::new(),
            c7_proofs_pending: None,
            last_ec: None,
        }
    }

    pub fn add_share(
        &mut self,
        party_id: u64,
        share: Vec<ArcBytes>,
        signed_decryption_proofs: Vec<SignedProofPayload>,
        wrapped_proofs: Vec<Proof>,
        ec: &EventContext<Sequenced>,
    ) -> Result<()> {
        self.state.try_mutate(ec, |state| {
            info!("Adding share for party_id={}", party_id);
            let current: Collecting = state.clone().try_into()?;
            let ciphertext_output = current.ciphertext_output;
            let threshold_m = current.threshold_m;
            let threshold_n = current.threshold_n;
            let params = current.params.clone();
            let mut shares = current.shares;
            let mut c6_proofs = current.c6_proofs;
            let mut c6_wrapped_proofs = current.c6_wrapped_proofs;

            info!("pushing to share collection {} {:?}", party_id, share);
            shares.insert(party_id, share);
            c6_proofs.insert(party_id, signed_decryption_proofs);
            c6_wrapped_proofs.insert(party_id, wrapped_proofs);

            if (shares.len() as u64) < threshold_n {
                return Ok(ThresholdPlaintextAggregatorState::Collecting(Collecting {
                    params,
                    threshold_n,
                    threshold_m,
                    ciphertext_output,
                    shares,
                    c6_proofs,
                    c6_wrapped_proofs,
                    seed: current.seed,
                }));
            }

            info!(
                "Changing state to VerifyingC6 because received all {} shares...",
                threshold_n
            );

            Ok(ThresholdPlaintextAggregatorState::VerifyingC6(
                VerifyingC6 {
                    shares,
                    c6_proofs,
                    c6_wrapped_proofs,
                    ciphertext_output,
                    threshold_m,
                    threshold_n,
                    params,
                },
            ))
        })
    }

    /// Dispatch C6 proof verification through ShareVerificationActor.
    pub fn dispatch_c6_verification(
        &mut self,
        c6_proofs: HashMap<u64, Vec<SignedProofPayload>>,
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
        let party_proofs: Vec<PartyProofsToVerify> = c6_proofs
            .into_iter()
            .map(|(party_id, signed_proofs)| PartyProofsToVerify {
                sender_party_id: party_id,
                signed_proofs,
            })
            .collect();

        self.bus.publish(
            ShareVerificationDispatched {
                e3_id: self.e3_id.clone(),
                kind: VerificationKind::ThresholdDecryptionProofs,
                share_proofs: party_proofs,
                decryption_proofs: vec![],
                pre_dishonest: BTreeSet::new(),
            },
            ec,
        )?;
        Ok(())
    }

    /// Handle ShareVerificationComplete for C6: filter dishonest parties, transition to Computing.
    pub fn handle_c6_verification_complete(
        &mut self,
        msg: TypedEvent<ShareVerificationComplete>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();

        if msg.kind != VerificationKind::ThresholdDecryptionProofs {
            return Ok(());
        }

        if msg.e3_id != self.e3_id {
            return Ok(());
        }

        let state: VerifyingC6 = self
            .state
            .get()
            .ok_or(anyhow!("Could not get state"))?
            .try_into()?;

        let dishonest_parties = &msg.dishonest_parties;
        if !dishonest_parties.is_empty() {
            warn!(
                "C6 verification: {} dishonest parties filtered: {:?}",
                dishonest_parties.len(),
                dishonest_parties
            );
        }

        // Filter shares to only honest parties
        let honest_shares: Vec<(u64, Vec<ArcBytes>)> = state
            .shares
            .iter()
            .filter(|(id, _)| !dishonest_parties.contains(id))
            .map(|(id, s)| (*id, s.clone()))
            .collect();

        ensure!(
            honest_shares.len() > state.threshold_m as usize,
            "Not enough honest shares after C6 verification: {} honest shares, {} required",
            honest_shares.len(),
            state.threshold_m + 1
        );

        info!(
            "C6 verification passed: {} honest parties, transitioning to Computing",
            honest_shares.len()
        );

        // Collect honest C6 wrapped proofs sorted by party_id for cross-node folding.
        let mut honest_c6_wrapped: Vec<(u64, Vec<Proof>)> = state
            .c6_wrapped_proofs
            .iter()
            .filter(|(id, _)| !dishonest_parties.contains(id))
            .map(|(id, proofs)| (*id, proofs.clone()))
            .collect();
        honest_c6_wrapped.sort_by_key(|(id, _)| *id);

        // Publish ComputeRequest before transitioning state so a publish
        // failure leaves us in VerifyingC6 (retryable) rather than
        // Computing (no retry path).
        let trbfv_config =
            TrBFVConfig::new(state.params.clone(), state.threshold_n, state.threshold_m);

        let event = ComputeRequest::trbfv(
            TrBFVRequest::CalculateThresholdDecryption(
                CalculateThresholdDecryptionRequest {
                    ciphertexts: state.ciphertext_output.clone(),
                    trbfv_config,
                    d_share_polys: honest_shares.clone(),
                }
                .into(),
            ),
            CorrelationId::new(),
            self.e3_id.clone(),
        );
        self.bus.publish(event, ec.clone())?;

        self.state.try_mutate(&ec, |_| {
            Ok(ThresholdPlaintextAggregatorState::Computing(Computing {
                shares: honest_shares,
                ciphertext_output: state.ciphertext_output,
                threshold_m: state.threshold_m,
                threshold_n: state.threshold_n,
                params: state.params,
            }))
        })?;

        // Start C6 cross-node fold concurrently with threshold decryption.
        self.last_ec = Some(ec.clone());
        let proofs: Vec<Proof> = honest_c6_wrapped
            .into_iter()
            .flat_map(|(_, proofs)| proofs)
            .collect();
        self.c6_fold.start(
            proofs,
            "ThresholdPlaintextAggregator C6 fold",
            &self.bus,
            &self.e3_id,
            &ec,
        )?;
        self.try_publish_complete()?;

        Ok(())
    }

    /// Publish AggregationProofPending for C7 proof generation through ProofRequestActor.
    pub fn dispatch_c7_proof_request(
        &mut self,
        shares: Vec<(u64, Vec<ArcBytes>)>,
        plaintext: Vec<ArcBytes>,
        threshold_m: u64,
        threshold_n: u64,
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
        self.bus.publish(
            AggregationProofPending {
                e3_id: self.e3_id.clone(),
                proof_request: DecryptedSharesAggregationProofRequest {
                    d_share_polys: shares.clone(),
                    plaintext: plaintext.clone(),
                    params_preset: self.params_preset.clone(),
                    threshold_m,
                    threshold_n,
                },
                plaintext,
                shares,
            },
            ec,
        )?;
        Ok(())
    }

    /// Handle AggregationProofSigned: store C7 proofs and wait for C6 fold before publishing.
    pub fn handle_aggregation_proof_signed(
        &mut self,
        msg: TypedEvent<AggregationProofSigned>,
    ) -> Result<()> {
        let (msg, ec) = msg.into_components();

        if msg.e3_id != self.e3_id {
            return Ok(());
        }

        let state: GeneratingC7Proof = self
            .state
            .get()
            .ok_or(anyhow!("Could not get state"))?
            .try_into()?;

        // Extract raw proofs from signed payloads for PlaintextAggregated
        let proofs: Vec<_> = msg
            .signed_proofs
            .iter()
            .map(|sp| sp.payload.proof.clone())
            .collect();

        ensure!(
            proofs.len() == state.plaintext.len(),
            "C7 proof count mismatch: got {} proofs for {} ciphertext indices",
            proofs.len(),
            state.plaintext.len()
        );

        info!("C7 proof signed — waiting for C6 cross-node fold to complete...");
        self.c7_proofs_pending = Some(proofs);
        self.last_ec = Some(ec);
        self.try_publish_complete()
    }

    pub fn handle_compute_response(&mut self, msg: TypedEvent<ComputeResponse>) -> Result<()> {
        let (msg, ec) = msg.into_components();
        ensure!(
            msg.e3_id == self.e3_id,
            "PlaintextAggregator should never receive incorrect e3_id msgs"
        );

        let correlation_id = msg.correlation_id;
        match msg.response {
            // TrBFV threshold decryption response -> transition to GeneratingC7Proof
            ComputeResponseKind::TrBFV(TrBFVResponse::CalculateThresholdDecryption(response)) => {
                info!("Received TrBFV threshold decryption response");
                let plaintext = response.plaintext;

                let state: Computing = self
                    .state
                    .get()
                    .ok_or(anyhow!("Could not get state"))?
                    .try_into()?;

                let shares = state.shares.clone();
                let threshold_m = state.threshold_m;
                let threshold_n = state.threshold_n;

                // Publish pending event before transitioning state so a publish
                // failure leaves us in Computing (retryable) rather than
                // GeneratingC7Proof (no retry path).
                self.dispatch_c7_proof_request(
                    shares.clone(),
                    plaintext.clone(),
                    threshold_m,
                    threshold_n,
                    ec.clone(),
                )?;

                // Transition to GeneratingC7Proof
                self.state.try_mutate(&ec, |_| {
                    Ok(ThresholdPlaintextAggregatorState::GeneratingC7Proof(
                        GeneratingC7Proof {
                            threshold_m,
                            threshold_n,
                            shares,
                            plaintext,
                        },
                    ))
                })?;
            }

            // C6 cross-node fold response (ignore unrelated FoldProofs, e.g. PK C5 fold on same bus)
            ComputeResponseKind::Zk(ZkResponse::FoldProofs(resp)) => {
                if self.c6_fold.awaits_correlation(&correlation_id) {
                    let fold_ec = self
                        .last_ec
                        .clone()
                        .unwrap_or_else(|| ec.clone());
                    if self.c6_fold.handle_response(
                        &correlation_id,
                        resp.proof,
                        "ThresholdPlaintextAggregator C6 fold",
                        &self.bus,
                        &self.e3_id,
                        &fold_ec,
                    )? {
                        self.try_publish_complete()?;
                    }
                }
            }

            _ => {
                // Not a response we handle — ignore
            }
        }

        Ok(())
    }

    /// Publish `PlaintextAggregated` when both C7 proofs and C6 fold are complete.
    fn try_publish_complete(&mut self) -> Result<()> {
        let Some(c7_proofs) = self.c7_proofs_pending.as_ref() else {
            return Ok(());
        };
        let c6_ready = self.c6_fold.result.is_some() || self.c6_fold.fold_input_was_empty;
        if !c6_ready {
            return Ok(());
        }

        let state: GeneratingC7Proof = self
            .state
            .get()
            .ok_or_else(|| anyhow!("Expected GeneratingC7Proof state"))?
            .try_into()?;

        let ec = self
            .last_ec
            .clone()
            .ok_or_else(|| anyhow!("No EventContext for publish"))?;

        info!("Both C7 and C6 fold proof ready — publishing PlaintextAggregated");

        let len = MAX_MSG_NON_ZERO_COEFFS * 8;
        let decrypted_output: Vec<ArcBytes> = state
            .plaintext
            .iter()
            .map(|pt| {
                let mut bytes = pt.extract_bytes();
                if bytes.len() >= len {
                    bytes.truncate(len);
                } else {
                    bytes.resize(len, 0);
                }
                ArcBytes::from_bytes(&bytes)
            })
            .collect();

        let event = PlaintextAggregated {
            decrypted_output,
            e3_id: self.e3_id.clone(),
            aggregation_proofs: c7_proofs.clone(),
            c6_aggregated_proof: self.c6_fold.result.clone(),
        };

        info!(
            "Dispatching plaintext event with C7 and C6 proofs {:?}",
            event
        );
        self.bus.publish(event, ec.clone())?;

        self.state.try_mutate(&ec, |_| {
            Ok(ThresholdPlaintextAggregatorState::Complete(Complete {
                decrypted: state.plaintext,
                shares: state.shares,
            }))
        })?;

        Ok(())
    }
}

impl Actor for ThresholdPlaintextAggregator {
    type Context = Context<Self>;
    fn started(&mut self, ctx: &mut Self::Context) {
        ctx.set_mailbox_capacity(MAILBOX_LIMIT);
    }
}

impl Handler<EnclaveEvent> for ThresholdPlaintextAggregator {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        let (msg, ec) = msg.into_components();
        match msg {
            EnclaveEventData::DecryptionshareCreated(data) => ctx.notify(TypedEvent::new(data, ec)),
            EnclaveEventData::E3RequestComplete(_) => self.notify_sync(ctx, Die),
            EnclaveEventData::ComputeResponse(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::ShareVerificationComplete(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            EnclaveEventData::AggregationProofSigned(data) => {
                self.notify_sync(ctx, TypedEvent::new(data, ec))
            }
            _ => (),
        }
    }
}

impl Handler<TypedEvent<DecryptionshareCreated>> for ThresholdPlaintextAggregator {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<DecryptionshareCreated>,
        ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::PublickeyAggregation,
            &self.bus.with_ec(msg.get_ctx()),
            || {
                let Some(ThresholdPlaintextAggregatorState::Collecting(Collecting { .. })) =
                    self.state.get()
                else {
                    debug!(state=?self.state, "Aggregator has been closed for collecting so ignoring this event.");
                    return Ok(());
                };
                let node = msg.node.clone();
                let e3_id = msg.e3_id.clone();
                let request = E3CommitteeContainsRequest::new(e3_id, node, msg, ctx.address());
                self.sortition.try_send(request)?;
                Ok(())
            },
        )
    }
}

impl Handler<E3CommitteeContainsResponse<TypedEvent<DecryptionshareCreated>>>
    for ThresholdPlaintextAggregator
{
    type Result = ();
    fn handle(
        &mut self,
        msg: E3CommitteeContainsResponse<TypedEvent<DecryptionshareCreated>>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::PublickeyAggregation,
            &self.bus.with_ec(msg.get_ctx()),
            || {
                let e3_id = &msg.e3_id;
                if *e3_id != self.e3_id {
                    bail!("Wrong e3_id sent to aggregator. This should not happen.")
                };

                if !msg.is_found_in_committee() {
                    trace!("Node {} not found in finalized committee", &msg.node);
                    return Ok(());
                };

                // Trust the party_id from the event - it's based on CommitteeFinalized order
                // which is the authoritative source of truth for party IDs
                let (
                    DecryptionshareCreated {
                        party_id,
                        decryption_share,
                        signed_decryption_proofs,
                        wrapped_proofs,
                        ..
                    },
                    ec,
                ) = msg.into_inner().into_components();

                self.add_share(
                    party_id,
                    decryption_share,
                    signed_decryption_proofs,
                    wrapped_proofs,
                    &ec,
                )?;

                // If we transitioned to VerifyingC6, dispatch C6 verification
                // using the proofs persisted in state
                if let Some(ThresholdPlaintextAggregatorState::VerifyingC6(ref state)) =
                    self.state.get()
                {
                    self.dispatch_c6_verification(state.c6_proofs.clone(), ec)?;
                }

                Ok(())
            },
        )
    }
}

impl Handler<TypedEvent<ComputeResponse>> for ThresholdPlaintextAggregator {
    type Result = ();
    fn handle(&mut self, msg: TypedEvent<ComputeResponse>, _: &mut Self::Context) -> Self::Result {
        trap(
            EType::PlaintextAggregation,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_compute_response(msg),
        )
    }
}

impl Handler<TypedEvent<ShareVerificationComplete>> for ThresholdPlaintextAggregator {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<ShareVerificationComplete>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::PlaintextAggregation,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_c6_verification_complete(msg),
        )
    }
}

impl Handler<TypedEvent<AggregationProofSigned>> for ThresholdPlaintextAggregator {
    type Result = ();
    fn handle(
        &mut self,
        msg: TypedEvent<AggregationProofSigned>,
        _ctx: &mut Self::Context,
    ) -> Self::Result {
        trap(
            EType::PlaintextAggregation,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_aggregation_proof_signed(msg),
        )
    }
}

impl Handler<Die> for ThresholdPlaintextAggregator {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop()
    }
}
