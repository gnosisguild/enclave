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
    ComputeRequest, DecryptionshareCreated, Die, E3id, EnclaveEvent, EventBus, PlaintextAggregated,
    Seed,
};
use e3_multithread::Multithread;
use e3_sortition::{GetNodeIndex, Sortition};
use e3_trbfv::{
    calculate_threshold_decryption::{
        CalculateThresholdDecryptionRequest, CalculateThresholdDecryptionResponse,
    },
    TrBFVConfig, TrBFVRequest,
};
use e3_utils::utility_types::ArcBytes;
use tracing::{error, info, warn};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Collecting {
    threshold_m: u64,
    threshold_n: u64,
    shares: HashMap<u64, Vec<ArcBytes>>,
    seed: Seed,
    ciphertext_output: ArcBytes,
    params: ArcBytes,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Computing {
    threshold_m: u64,
    threshold_n: u64,
    shares: Vec<(u64, Vec<ArcBytes>)>,
    ciphertext_output: ArcBytes,
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
        ciphertext_output: ArcBytes,
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
    pub ciphertext_output: ArcBytes,
    pub threshold_m: u64,
    pub threshold_n: u64,
}

pub struct ThresholdPlaintextAggregator {
    multithread: Addr<Multithread>,
    bus: Addr<EventBus<EnclaveEvent>>,
    sortition: Addr<Sortition>,
    e3_id: E3id,
    state: Persistable<ThresholdPlaintextAggregatorState>,
}

pub struct ThresholdPlaintextAggregatorParams {
    pub multithread: Addr<Multithread>,
    pub bus: Addr<EventBus<EnclaveEvent>>,
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
        match msg {
            EnclaveEvent::DecryptionshareCreated { data, .. } => ctx.notify(data),
            EnclaveEvent::E3RequestComplete { .. } => ctx.notify(Die),
            _ => (),
        }
    }
}

impl Handler<DecryptionshareCreated> for ThresholdPlaintextAggregator {
    type Result = ResponseActFuture<Self, Result<()>>;

    fn handle(&mut self, event: DecryptionshareCreated, _: &mut Self::Context) -> Self::Result {
        info!(event=?event, "Processing DecryptionShareCreated...");
        let Some(ThresholdPlaintextAggregatorState::Collecting(Collecting {
            threshold_n,
            seed,
            ..
        })) = self.state.get()
        else {
            error!(state=?self.state, "Aggregator has been closed for collecting.");
            return Box::pin(fut::ready(Ok(())));
        };

        let size = threshold_n as usize;
        let address = event.node;
        let party_id = event.party_id;
        let chain_id = event.e3_id.chain_id();
        let e3_id = event.e3_id.clone();
        let decryption_share = event.decryption_share.clone();

        // Why do we need to get the node index when the event contains the party_id? I guess we
        // don't trust the event. Maybe that is fine.
        Box::pin(
            self.sortition
                .send(GetNodeIndex {
                    chain_id,
                    address: address.clone(),
                    size,
                    seed,
                })
                .into_actor(self)
                .map(move |res, act, ctx| {
                    let maybe_found_index = res?;
                    let Some(party) = maybe_found_index else {
                        error!("Attempting to aggregate share but party not found in committee");
                        return Ok(());
                    };

                    if party != party_id {
                        error!(
                            "Bad aggregation state! Address {} not found at index {} instead it was found at {}",
                            address, party_id, party
                        );
                        return Ok(());
                    }

                    if e3_id != act.e3_id {
                        error!("Wrong e3_id sent to aggregator. This should not happen.");
                        return Ok(());
                    }

                    // add the keyshare and
                    act.add_share(party_id, decryption_share)?;

                    // Check the state and if it has changed to the computing
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
                    let event = EnclaveEvent::from(PlaintextAggregated {
                        decrypted_output: plaintext, // Extracting here for now
                        e3_id: act.e3_id.clone(),
                    });

                    info!("Dispatching plaintext event {:?}", event);
                    act.bus.do_send(event);
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
