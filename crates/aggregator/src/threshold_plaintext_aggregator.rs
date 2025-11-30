// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use std::collections::HashMap;

use actix::prelude::*;
use anyhow::{anyhow, bail, Result};
use e3_data::Persistable;
use e3_events::{
    prelude::*, BusHandle, ComputeRequest, DecryptionshareCreated, Die, E3id, EnclaveEvent,
    EnclaveEventData, PlaintextAggregated, Seed,
};
use e3_multithread::Multithread;
use e3_sortition::{GetNodesForE3, Sortition};
use e3_trbfv::{
    calculate_threshold_decryption::{
        CalculateThresholdDecryptionRequest, CalculateThresholdDecryptionResponse,
    },
    TrBFVConfig, TrBFVRequest,
};
use e3_utils::utility_types::ArcBytes;
use tracing::{debug, error, info, trace};

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
#[rtype(result = "anyhow::Result<()>")]
pub struct ComputeAggregate {
    pub shares: Vec<(u64, Vec<ArcBytes>)>,
    pub ciphertext_output: Vec<ArcBytes>,
    pub threshold_m: u64,
    pub threshold_n: u64,
}

pub struct ThresholdPlaintextAggregator {
    multithread: Addr<Multithread>,
    bus: BusHandle<EnclaveEvent>,
    sortition: Addr<Sortition>,
    e3_id: E3id,
    state: Persistable<ThresholdPlaintextAggregatorState>,
}

pub struct ThresholdPlaintextAggregatorParams {
    pub multithread: Addr<Multithread>,
    pub bus: BusHandle<EnclaveEvent>,
    pub sortition: Addr<Sortition>,
    pub e3_id: E3id,
}

impl ThresholdPlaintextAggregator {
    pub fn new(
        params: ThresholdPlaintextAggregatorParams,
        state: Persistable<ThresholdPlaintextAggregatorState>,
    ) -> Self {
        ThresholdPlaintextAggregator {
            multithread: params.multithread,
            bus: params.bus,
            sortition: params.sortition,
            e3_id: params.e3_id,
            state,
        }
    }

    pub fn add_share(&mut self, party_id: u64, share: Vec<ArcBytes>) -> Result<()> {
        self.state.try_mutate(|state| {
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

    pub fn set_decryption(&mut self, decrypted: Vec<ArcBytes>) -> Result<()> {
        self.state.try_mutate(|mut state| {
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

    pub fn create_calculate_threshold_decryption_event(
        &self,
        msg: ComputeAggregate,
    ) -> Result<ComputeRequest> {
        info!("create_calculate_threshold_decryption_event...");

        let state: Computing = self
            .state
            .get()
            .ok_or(anyhow!("Could not get state"))?
            .try_into()?;

        let trbfv_config =
            TrBFVConfig::new(state.params.clone(), state.threshold_n, state.threshold_m);

        Ok(ComputeRequest::TrBFV(
            TrBFVRequest::CalculateThresholdDecryption(
                CalculateThresholdDecryptionRequest {
                    ciphertexts: msg.ciphertext_output,
                    trbfv_config,
                    d_share_polys: msg.shares,
                }
                .into(),
            ),
        ))
    }
}

impl Actor for ThresholdPlaintextAggregator {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for ThresholdPlaintextAggregator {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg.into_data() {
            EnclaveEventData::DecryptionshareCreated(data) => ctx.notify(data),
            EnclaveEventData::E3RequestComplete(_) => ctx.notify(Die),
            _ => (),
        }
    }
}

impl Handler<DecryptionshareCreated> for ThresholdPlaintextAggregator {
    type Result = ResponseActFuture<Self, Result<()>>;

    fn handle(&mut self, event: DecryptionshareCreated, _: &mut Self::Context) -> Self::Result {
        let Some(ThresholdPlaintextAggregatorState::Collecting(Collecting { .. })) =
            self.state.get()
        else {
            debug!(state=?self.state, "Aggregator has been closed for collecting so ignoring this event.");
            return Box::pin(fut::ready(Ok(())));
        };
        info!(event=?event, "Processing DecryptionShareCreated...");
        let address = event.node.clone();
        let party_id = event.party_id;
        let e3_id = event.e3_id.clone();
        let decryption_share = event.decryption_share.clone();

        Box::pin(
            self.sortition
                .send(GetNodesForE3 {
                    e3_id: e3_id.clone(),
                    chain_id: e3_id.chain_id(),
                })
                .into_actor(self)
                .map(move |res, act, ctx| {
                    let nodes = res?;

                    if !nodes.contains(&address) {
                        trace!("Node {} not found in finalized committee", address);
                        return Ok(());
                    }

                    if e3_id != act.e3_id {
                        error!("Wrong e3_id sent to aggregator. This should not happen.");
                        return Ok(());
                    }

                    // Trust the party_id from the event - it's based on CommitteeFinalized order
                    // which is the authoritative source of truth for party IDs
                    act.add_share(party_id, decryption_share)?;

                    if let Some(ThresholdPlaintextAggregatorState::Computing(Computing {
                        threshold_m,
                        threshold_n,
                        shares,
                        ciphertext_output,
                        ..
                    })) = act.state.get()
                    {
                        ctx.notify(ComputeAggregate {
                            shares: shares.clone(),
                            ciphertext_output: ciphertext_output.clone(),
                            threshold_m,
                            threshold_n,
                        })
                    }

                    Ok(())
                }),
        )
    }
}

impl Handler<ComputeAggregate> for ThresholdPlaintextAggregator {
    type Result = ResponseActFuture<Self, Result<()>>;
    fn handle(&mut self, msg: ComputeAggregate, _: &mut Self::Context) -> Self::Result {
        let event = match self.create_calculate_threshold_decryption_event(msg) {
            Ok(event) => event,
            Err(e) => {
                error!("{e}");
                return e3_utils::actix::bail_result(self, "{e}");
            }
        };
        Box::pin(
            self.multithread
                .send(event)
                .into_actor(self)
                .map(move |res, act, _| {
                    let response: CalculateThresholdDecryptionResponse = match res? {
                        Ok(res) => res.try_into()?,
                        Err(e) => {
                            error!("{e}");
                            bail!(e)
                        }
                    };

                    info!("Received response {:?}", response);

                    // Update the local state
                    let plaintext = response.plaintext;

                    act.set_decryption(plaintext.clone())?;

                    // Dispatch the PlaintextAggregated event
                    let event = PlaintextAggregated {
                        decrypted_output: plaintext, // Extracting here for now
                        e3_id: act.e3_id.clone(),
                    };

                    info!("Dispatching plaintext event {:?}", event);
                    act.bus.dispatch(event);
                    Ok(())
                }),
        )
    }
}

impl Handler<Die> for ThresholdPlaintextAggregator {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop()
    }
}
