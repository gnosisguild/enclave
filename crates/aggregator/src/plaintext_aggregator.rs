// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use anyhow::Result;
use e3_data::Persistable;
use e3_events::{
    DecryptionshareCreated, Die, E3id, EnclaveEvent, EventBus, OrderedSet, PlaintextAggregated,
    Seed,
};
use e3_fhe::{Fhe, GetAggregatePlaintext};
use e3_sortition::{GetNodeIndex, Sortition};
use e3_utils::ArcBytes;
use std::sync::Arc;
use tracing::{error, warn};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PlaintextAggregatorState {
    Collecting {
        threshold_m: usize,
        threshold_n: usize,
        shares: OrderedSet<Vec<u8>>,
        seed: Seed,
        ciphertext_output: ArcBytes,
    },
    Computing {
        shares: OrderedSet<Vec<u8>>,
        ciphertext_output: ArcBytes,
    },
    Complete {
        decrypted: Vec<u8>,
        shares: OrderedSet<Vec<u8>>,
    },
}

impl PlaintextAggregatorState {
    pub fn init(
        threshold_m: usize,
        threshold_n: usize,
        seed: Seed,
        ciphertext_output: ArcBytes,
    ) -> Self {
        PlaintextAggregatorState::Collecting {
            threshold_m,
            threshold_n,
            shares: OrderedSet::new(),
            seed,
            ciphertext_output,
        }
    }

    pub fn get_name(&self) -> String {
        match self {
            PlaintextAggregatorState::Collecting { .. } => "Collecting",
            PlaintextAggregatorState::Computing { .. } => "Computing",
            PlaintextAggregatorState::Complete { .. } => "Complete",
        }
        .to_string()
    }
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<()>")]
struct ComputeAggregate {
    pub shares: OrderedSet<Vec<u8>>,
    pub ciphertext_output: Vec<u8>,
}

#[deprecated = "To be replaced by ThresholdPlaintextAggregator"]
pub struct PlaintextAggregator {
    fhe: Arc<Fhe>,
    bus: Addr<EventBus<EnclaveEvent>>,
    sortition: Addr<Sortition>,
    e3_id: E3id,
    state: Persistable<PlaintextAggregatorState>,
}

pub struct PlaintextAggregatorParams {
    pub fhe: Arc<Fhe>,
    pub bus: Addr<EventBus<EnclaveEvent>>,
    pub sortition: Addr<Sortition>,
    pub e3_id: E3id,
}

impl PlaintextAggregator {
    pub fn new(
        params: PlaintextAggregatorParams,
        state: Persistable<PlaintextAggregatorState>,
    ) -> Self {
        PlaintextAggregator {
            fhe: params.fhe,
            bus: params.bus,
            sortition: params.sortition,
            e3_id: params.e3_id,
            state,
        }
    }

    pub fn add_share(&mut self, share: Vec<u8>) -> Result<()> {
        self.state.try_mutate(|mut state| {
            let PlaintextAggregatorState::Collecting {
                // NOTE: In the deprecated PlaintextAggregator we need all shares to
                // decrypt so here we set threshold_n
                threshold_n,
                shares,
                ciphertext_output,
                ..
            } = &mut state
            else {
                return Err(anyhow::anyhow!("Can only add share in Collecting state"));
            };

            shares.insert(share);

            if shares.len() == *threshold_n {
                return Ok(PlaintextAggregatorState::Computing {
                    shares: shares.clone(),
                    ciphertext_output: ciphertext_output.clone(),
                });
            }

            Ok(state)
        })
    }

    pub fn set_decryption(&mut self, decrypted: Vec<u8>) -> Result<()> {
        self.state.try_mutate(|mut state| {
            let PlaintextAggregatorState::Computing { shares, .. } = &mut state else {
                return Ok(state.clone());
            };
            let shares = shares.to_owned();

            Ok(PlaintextAggregatorState::Complete { decrypted, shares })
        })
    }
}

impl Actor for PlaintextAggregator {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for PlaintextAggregator {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::DecryptionshareCreated { data, .. } => ctx.notify(data),
            EnclaveEvent::E3RequestComplete { .. } => ctx.notify(Die),
            _ => (),
        }
    }
}

impl Handler<DecryptionshareCreated> for PlaintextAggregator {
    type Result = ResponseActFuture<Self, Result<()>>;

    fn handle(&mut self, event: DecryptionshareCreated, _: &mut Self::Context) -> Self::Result {
        let Some(PlaintextAggregatorState::Collecting {
            threshold_n, seed, ..
        }) = self.state.get()
        else {
            let name = self.state.get().map(|s| s.get_name());
            error!(
                "Aggregator has been closed for collecting. {}",
                name.unwrap_or("Unknown".to_string())
            );
            return Box::pin(fut::ready(Ok(())));
        };

        let size = threshold_n;
        let address = event.node;
        let chain_id = event.e3_id.chain_id();
        let e3_id = event.e3_id.clone();
        let decryption_share = event.decryption_share.clone();

        Box::pin(
            self.sortition
                .send(GetNodeIndex {
                    chain_id,
                    address,
                    size,
                    seed,
                })
                .into_actor(self)
                .map(move |res, act, ctx| {
                    let maybe_found_index = res?;
                    let Some(_) = maybe_found_index else {
                        error!("Node not found in committee");
                        return Ok(());
                    };

                    if e3_id != act.e3_id {
                        error!("Wrong e3_id sent to aggregator. This should not happen.");
                        return Ok(());
                    }

                    // add the keyshare and
                    let Some(share) = decryption_share.first() else {
                        error!("Share not found in decryption_share vector");
                        return Ok(());
                    };

                    act.add_share(share.extract_bytes())?;

                    // Check the state and if it has changed to the computing
                    if let Some(PlaintextAggregatorState::Computing {
                        shares,
                        ciphertext_output,
                    }) = &act.state.get()
                    {
                        ctx.notify(ComputeAggregate {
                            shares: shares.clone(),
                            ciphertext_output: ciphertext_output.to_vec(),
                        })
                    }

                    Ok(())
                }),
        )
    }
}

impl Handler<ComputeAggregate> for PlaintextAggregator {
    type Result = Result<()>;
    fn handle(&mut self, msg: ComputeAggregate, _: &mut Self::Context) -> Self::Result {
        let decrypted_output = self.fhe.get_aggregate_plaintext(GetAggregatePlaintext {
            decryptions: msg.shares.clone(),
            ciphertext_output: msg.ciphertext_output,
        })?;

        // Update the local state
        self.set_decryption(decrypted_output.clone())?;
        // Dispatch the PlaintextAggregated event
        let event = EnclaveEvent::from(PlaintextAggregated {
            decrypted_output: ArcBytes::from_bytes(decrypted_output),
            e3_id: self.e3_id.clone(),
        });

        self.bus.do_send(event);

        Ok(())
    }
}

impl Handler<Die> for PlaintextAggregator {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop()
    }
}
