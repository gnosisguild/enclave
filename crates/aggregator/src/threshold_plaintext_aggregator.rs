// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::collections::HashMap;

use actix::prelude::*;
use anyhow::{anyhow, bail, ensure, Result};
use e3_data::Persistable;
use e3_events::{
    prelude::*, trap, BusHandle, ComputeRequest, ComputeResponse, ComputeResponseKind,
    CorrelationId, DecryptedSharesAggregationProofRequest, DecryptedSharesAggregationProofResponse,
    DecryptionshareCreated, Die, E3id, EType, EnclaveEvent, EnclaveEventData, EventContext,
    PartyC6ProofsToVerify, PlaintextAggregated, Proof, Seed, Sequenced, TypedEvent,
    VerifyC6ProofsRequest, VerifyC6ProofsResponse, ZkRequest, ZkResponse,
};
use e3_fhe_params::BfvPreset;
use e3_sortition::{E3CommitteeContainsRequest, E3CommitteeContainsResponse, Sortition};
use e3_trbfv::{
    calculate_threshold_decryption::CalculateThresholdDecryptionRequest, TrBFVConfig, TrBFVRequest,
    TrBFVResponse,
};
use e3_utils::NotifySync;
use e3_utils::{utility_types::ArcBytes, MAILBOX_LIMIT};
use tracing::{debug, info, trace, warn};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Collecting {
    threshold_m: u64,
    threshold_n: u64,
    shares: HashMap<u64, Vec<ArcBytes>>,
    c6_proofs: HashMap<u64, Vec<Proof>>,
    seed: Seed,
    ciphertext_output: Vec<ArcBytes>,
    params: ArcBytes,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VerifyingC6 {
    threshold_m: u64,
    threshold_n: u64,
    shares: HashMap<u64, Vec<ArcBytes>>,
    c6_proofs: HashMap<u64, Vec<Proof>>,
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
            seed,
            ciphertext_output,
            params,
        })
    }
}

#[derive(Message)]
#[rtype("()")]
pub struct ComputeAggregate {
    pub shares: Vec<(u64, Vec<ArcBytes>)>,
    pub ciphertext_output: Vec<ArcBytes>,
    pub threshold_m: u64,
    pub threshold_n: u64,
}

pub struct ThresholdPlaintextAggregator {
    bus: BusHandle,
    sortition: Addr<Sortition>,
    e3_id: E3id,
    params_preset: BfvPreset,
    state: Persistable<ThresholdPlaintextAggregatorState>,
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
        }
    }

    pub fn add_share(
        &mut self,
        party_id: u64,
        share: Vec<ArcBytes>,
        decryption_proofs: Vec<Proof>,
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

            info!("pushing to share collection {} {:?}", party_id, share);
            shares.insert(party_id, share);
            c6_proofs.insert(party_id, decryption_proofs);

            if shares.len() <= threshold_m as usize {
                return Ok(ThresholdPlaintextAggregatorState::Collecting(Collecting {
                    params,
                    threshold_n,
                    threshold_m,
                    ciphertext_output,
                    shares,
                    c6_proofs,
                    seed: current.seed,
                }));
            }

            info!("Changing state to VerifyingC6 because received enough shares...");

            Ok(ThresholdPlaintextAggregatorState::VerifyingC6(
                VerifyingC6 {
                    shares,
                    c6_proofs,
                    ciphertext_output,
                    threshold_m,
                    threshold_n,
                    params,
                },
            ))
        })
    }

    /// Dispatch a C6 proof verification request for all collected parties.
    pub fn dispatch_c6_verification(
        &mut self,
        c6_proofs: HashMap<u64, Vec<Proof>>,
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
        let party_proofs: Vec<PartyC6ProofsToVerify> = c6_proofs
            .into_iter()
            .map(|(party_id, proofs)| PartyC6ProofsToVerify {
                sender_party_id: party_id,
                c6_proofs: proofs,
            })
            .collect();

        let event = ComputeRequest::zk(
            ZkRequest::VerifyC6Proofs(VerifyC6ProofsRequest { party_proofs }),
            CorrelationId::new(),
            self.e3_id.clone(),
        );

        self.bus.publish(event, ec)?;
        Ok(())
    }

    /// Handle C6 verification response: filter dishonest parties, transition to Computing.
    pub fn handle_c6_verification_response(
        &mut self,
        data: VerifyC6ProofsResponse,
        ec: &EventContext<Sequenced>,
    ) -> Result<()> {
        let state: VerifyingC6 = self
            .state
            .get()
            .ok_or(anyhow!("Could not get state"))?
            .try_into()?;

        // Determine honest parties
        let honest_party_ids: Vec<u64> = data
            .party_results
            .iter()
            .filter(|r| r.all_verified)
            .map(|r| r.sender_party_id)
            .collect();

        let dishonest: Vec<u64> = data
            .party_results
            .iter()
            .filter(|r| !r.all_verified)
            .map(|r| r.sender_party_id)
            .collect();

        if !dishonest.is_empty() {
            warn!(
                "C6 verification: {} dishonest parties filtered: {:?}",
                dishonest.len(),
                dishonest
            );
        }

        // Filter shares to only honest parties
        let honest_shares: Vec<(u64, Vec<ArcBytes>)> = honest_party_ids
            .iter()
            .filter_map(|id| state.shares.get(id).map(|s| (*id, s.clone())))
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

        self.state.try_mutate(ec, |_| {
            Ok(ThresholdPlaintextAggregatorState::Computing(Computing {
                shares: honest_shares.clone(),
                ciphertext_output: state.ciphertext_output.clone(),
                threshold_m: state.threshold_m,
                threshold_n: state.threshold_n,
                params: state.params.clone(),
            }))
        })?;

        Ok(())
    }

    pub fn handle_compute_aggregate(&mut self, msg: TypedEvent<ComputeAggregate>) -> Result<()> {
        let (msg, ec) = msg.into_components();
        info!("create_calculate_threshold_decryption_event...");

        let e3_id = self.e3_id.clone();
        let state: Computing = self
            .state
            .get()
            .ok_or(anyhow!("Could not get state"))?
            .try_into()?;

        let trbfv_config =
            TrBFVConfig::new(state.params.clone(), state.threshold_n, state.threshold_m);

        let event = ComputeRequest::trbfv(
            TrBFVRequest::CalculateThresholdDecryption(
                CalculateThresholdDecryptionRequest {
                    ciphertexts: msg.ciphertext_output,
                    trbfv_config,
                    d_share_polys: msg.shares,
                }
                .into(),
            ),
            CorrelationId::new(),
            e3_id,
        );
        self.bus.publish(event, ec)?;
        Ok(())
    }

    /// Dispatch a C7 proof generation request.
    pub fn dispatch_c7_proof_request(
        &mut self,
        shares: Vec<(u64, Vec<ArcBytes>)>,
        plaintext: Vec<ArcBytes>,
        threshold_m: u64,
        threshold_n: u64,
        ec: EventContext<Sequenced>,
    ) -> Result<()> {
        let event = ComputeRequest::zk(
            ZkRequest::DecryptedSharesAggregation(DecryptedSharesAggregationProofRequest {
                d_share_polys: shares,
                plaintext,
                params_preset: self.params_preset.clone(),
                threshold_m,
                threshold_n,
            }),
            CorrelationId::new(),
            self.e3_id.clone(),
        );

        self.bus.publish(event, ec)?;
        Ok(())
    }

    /// Handle C7 proof response: transition to Complete and publish PlaintextAggregated.
    pub fn handle_c7_proof_response(
        &mut self,
        data: DecryptedSharesAggregationProofResponse,
        ec: &EventContext<Sequenced>,
    ) -> Result<()> {
        let state: GeneratingC7Proof = self
            .state
            .get()
            .ok_or(anyhow!("Could not get state"))?
            .try_into()?;

        let plaintext = state.plaintext.clone();
        let shares = state.shares.clone();

        ensure!(
            data.proofs.len() == plaintext.len(),
            "C7 proof count mismatch: got {} proofs for {} ciphertext indices",
            data.proofs.len(),
            plaintext.len()
        );

        self.state.try_mutate(ec, |_| {
            Ok(ThresholdPlaintextAggregatorState::Complete(Complete {
                decrypted: plaintext.clone(),
                shares: shares.clone(),
            }))
        })?;

        // Dispatch the PlaintextAggregated event with C7 proofs
        let event = PlaintextAggregated {
            decrypted_output: plaintext,
            e3_id: self.e3_id.clone(),
            aggregation_proofs: data.proofs,
        };

        info!("Dispatching plaintext event with C7 proofs {:?}", event);
        self.bus.publish(event, ec.clone())?;
        Ok(())
    }

    pub fn handle_compute_response(&mut self, msg: TypedEvent<ComputeResponse>) -> Result<()> {
        let (msg, ec) = msg.into_components();
        ensure!(
            msg.e3_id == self.e3_id,
            "PlaintextAggregator should never receive incorrect e3_id msgs"
        );

        match msg.response {
            // C6 verification response
            ComputeResponseKind::Zk(ZkResponse::VerifyC6Proofs(data)) => {
                info!("Received C6 verification response");
                self.handle_c6_verification_response(data, &ec)?;

                // Now dispatch the TrBFV computation
                let state: Computing = self
                    .state
                    .get()
                    .ok_or(anyhow!("Could not get state after C6 verification"))?
                    .try_into()?;

                let trbfv_config =
                    TrBFVConfig::new(state.params.clone(), state.threshold_n, state.threshold_m);

                let event = ComputeRequest::trbfv(
                    TrBFVRequest::CalculateThresholdDecryption(
                        CalculateThresholdDecryptionRequest {
                            ciphertexts: state.ciphertext_output.clone(),
                            trbfv_config,
                            d_share_polys: state.shares.clone(),
                        }
                        .into(),
                    ),
                    CorrelationId::new(),
                    self.e3_id.clone(),
                );
                self.bus.publish(event, ec)?;
            }

            // TrBFV threshold decryption response → transition to GeneratingC7Proof
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

                // Transition to GeneratingC7Proof
                self.state.try_mutate(&ec, |_| {
                    Ok(ThresholdPlaintextAggregatorState::GeneratingC7Proof(
                        GeneratingC7Proof {
                            threshold_m,
                            threshold_n,
                            shares: shares.clone(),
                            plaintext: plaintext.clone(),
                        },
                    ))
                })?;

                // Dispatch C7 proof request
                self.dispatch_c7_proof_request(shares, plaintext, threshold_m, threshold_n, ec)?;
            }

            // C7 proof response → Complete + publish
            ComputeResponseKind::Zk(ZkResponse::DecryptedSharesAggregation(data)) => {
                info!("Received C7 proof response");
                self.handle_c7_proof_response(data, &ec)?;
            }

            _ => {
                // Not a response we handle — ignore
            }
        }

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
                        decryption_proofs,
                        ..
                    },
                    ec,
                ) = msg.into_inner().into_components();

                self.add_share(party_id, decryption_share, decryption_proofs, &ec)?;

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

impl Handler<TypedEvent<ComputeAggregate>> for ThresholdPlaintextAggregator {
    type Result = ();
    fn handle(&mut self, msg: TypedEvent<ComputeAggregate>, _: &mut Self::Context) -> Self::Result {
        trap(
            EType::PlaintextAggregation,
            &self.bus.with_ec(msg.get_ctx()),
            || self.handle_compute_aggregate(msg),
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

impl Handler<Die> for ThresholdPlaintextAggregator {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop()
    }
}
