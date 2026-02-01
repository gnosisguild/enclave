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
    prelude::*, trap, BusHandle, ComputeRequest, ComputeResponse, CorrelationId,
    DecryptionshareCreated, Die, E3id, EType, EnclaveEvent, EnclaveEventData, EventContext,
    PlaintextAggregated, Seed, Sequenced, TypedEvent,
};
use e3_sortition::{E3CommitteeContainsRequest, E3CommitteeContainsResponse, Sortition};
use e3_trbfv::{
    calculate_threshold_decryption::CalculateThresholdDecryptionRequest, TrBFVConfig, TrBFVRequest,
    TrBFVResponse,
};
use e3_utils::utility_types::ArcBytes;
use e3_utils::NotifySync;
use tracing::{debug, info, trace};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Collecting {
    threshold_m: u64,
    threshold_n: u64,
    shares: HashMap<u64, Vec<ArcBytes>>,
    seed: Seed,
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
pub struct Complete {
    decrypted: Vec<ArcBytes>,
    shares: Vec<(u64, Vec<ArcBytes>)>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum ThresholdPlaintextAggregatorState {
    Collecting(Collecting),
    Computing(Computing),
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

impl TryFrom<ThresholdPlaintextAggregatorState> for Computing {
    type Error = anyhow::Error;
    fn try_from(
        value: ThresholdPlaintextAggregatorState,
    ) -> std::result::Result<Self, Self::Error> {
        match value {
            ThresholdPlaintextAggregatorState::Computing(s) => Ok(s),
            _ => bail!("Inconsistent state"),
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
            _ => bail!("Inconsistent state"),
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
    state: Persistable<ThresholdPlaintextAggregatorState>,
}

pub struct ThresholdPlaintextAggregatorParams {
    pub bus: BusHandle,
    pub sortition: Addr<Sortition>,
    pub e3_id: E3id,
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
            state,
        }
    }

    pub fn add_share(
        &mut self,
        party_id: u64,
        share: Vec<ArcBytes>,
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

            info!("pushing to share collection {} {:?}", party_id, share);
            shares.insert(party_id, share);

            if shares.len() <= threshold_m as usize {
                return Ok(ThresholdPlaintextAggregatorState::Collecting(Collecting {
                    params,
                    threshold_n,
                    threshold_m,
                    ciphertext_output,
                    shares,
                    seed: current.seed,
                }));
            }

            info!("Changing state to computing because received enough shares...");

            Ok(ThresholdPlaintextAggregatorState::Computing(Computing {
                shares: shares.into_iter().collect(),
                ciphertext_output,
                threshold_m,
                threshold_n,
                params,
            }))
        })
    }

    pub fn set_decryption(
        &mut self,
        decrypted: Vec<ArcBytes>,
        ec: &EventContext<Sequenced>,
    ) -> Result<()> {
        self.state.try_mutate(ec, |mut state| {
            let ThresholdPlaintextAggregatorState::Computing(Computing { shares, .. }) = &mut state
            else {
                return Ok(state.clone());
            };
            let shares = shares.to_owned();

            Ok(ThresholdPlaintextAggregatorState::Complete(Complete {
                decrypted,
                shares,
            }))
        })
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

        let event = ComputeRequest::new(
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

    pub fn handle_compute_response(&mut self, msg: TypedEvent<ComputeResponse>) -> Result<()> {
        let (msg, ec) = msg.into_components();
        ensure!(
            msg.e3_id == self.e3_id,
            "PlaintextAggregator should never receive incorrect e3_id msgs"
        );

        let TrBFVResponse::CalculateThresholdDecryption(response) = msg.response else {
            // Must be another compute response so ignoring
            return Ok(());
        };

        info!("Received response {:?}", response);

        // Update the local state
        let plaintext = response.plaintext;

        self.set_decryption(plaintext.clone(), &ec)?;

        // Dispatch the PlaintextAggregated event
        let event = PlaintextAggregated {
            decrypted_output: plaintext, // Extracting here for now
            e3_id: self.e3_id.clone(),
        };

        info!("Dispatching plaintext event {:?}", event);
        self.bus.publish(event, ec)?;
        Ok(())
    }
}

impl Actor for ThresholdPlaintextAggregator {
    type Context = Context<Self>;
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
        ctx: &mut Self::Context,
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
                        ..
                    },
                    ec,
                ) = msg.into_inner().into_components();

                self.add_share(party_id, decryption_share, &ec)?;

                if let Some(ThresholdPlaintextAggregatorState::Computing(Computing {
                    threshold_m,
                    threshold_n,
                    shares,
                    ciphertext_output,
                    ..
                })) = self.state.get()
                {
                    self.notify_sync(
                        ctx,
                        TypedEvent::new(
                            ComputeAggregate {
                                shares: shares.clone(),
                                ciphertext_output: ciphertext_output.clone(),
                                threshold_m,
                                threshold_n,
                            },
                            ec,
                        ),
                    )
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
