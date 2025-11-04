// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use anyhow::Result;
use e3_data::Persistable;
use e3_events::{
    Die, E3id, EnclaveEvent, EventBus, KeyshareCreated, OrderedSet, PublicKeyAggregated, Seed,
};
use e3_fhe::{Fhe, GetAggregatePublicKey};
use e3_sortition::{GetNodesForE3, Sortition};
use e3_utils::ArcBytes;
use std::{hash::Hash, sync::Arc};
use tracing::{error, info, trace};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PublicKeyAggregatorState {
    Collecting {
        threshold_n: usize,
        keyshares: OrderedSet<ArcBytes>,
        seed: Seed,
    },
    Computing {
        keyshares: OrderedSet<ArcBytes>,
    },
    Complete {
        public_key: Vec<u8>,
        keyshares: OrderedSet<ArcBytes>,
    },
}

impl PublicKeyAggregatorState {
    pub fn init(threshold_n: usize, seed: Seed) -> Self {
        PublicKeyAggregatorState::Collecting {
            threshold_n,
            keyshares: OrderedSet::new(),
            seed,
        }
    }
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<()>")]
struct ComputeAggregate {
    pub keyshares: OrderedSet<ArcBytes>,
    pub e3_id: E3id,
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<()>")]
struct NotifyNetwork {
    pub pubkey: Vec<u8>,
    pub e3_id: E3id,
}

pub struct PublicKeyAggregator {
    fhe: Arc<Fhe>,
    bus: Addr<EventBus<EnclaveEvent>>,
    sortition: Addr<Sortition>,
    e3_id: E3id,
    state: Persistable<PublicKeyAggregatorState>,
}

pub struct PublicKeyAggregatorParams {
    pub fhe: Arc<Fhe>,
    pub bus: Addr<EventBus<EnclaveEvent>>,
    pub sortition: Addr<Sortition>,
    pub e3_id: E3id,
}

/// Aggregate PublicKey for a committee of nodes. This actor listens for KeyshareCreated events
/// around a particular e3_id and aggregates the public key based on this and once done broadcasts
/// a EnclaveEvent::PublicKeyAggregated event on the event bus. Note events are hashed and
/// identical events will not be triggered twice.
/// It is expected to change this mechanism as we work through adversarial scenarios and write tests
/// for them.
impl PublicKeyAggregator {
    pub fn new(
        params: PublicKeyAggregatorParams,
        state: Persistable<PublicKeyAggregatorState>,
    ) -> Self {
        PublicKeyAggregator {
            fhe: params.fhe,
            bus: params.bus,
            sortition: params.sortition,
            e3_id: params.e3_id,
            state,
        }
    }

    pub fn add_keyshare(&mut self, keyshare: ArcBytes) -> Result<()> {
        self.state.try_mutate(|mut state| {
            let PublicKeyAggregatorState::Collecting {
                threshold_n,
                keyshares,
                ..
            } = &mut state
            else {
                return Err(anyhow::anyhow!("Can only add keyshare in Collecting state"));
            };

            keyshares.insert(keyshare);

            info!(
                "PublicKeyAggregator: Have {} keyshares waiting for {}",
                keyshares.len(),
                threshold_n
            );
            if keyshares.len() == *threshold_n {
                return Ok(PublicKeyAggregatorState::Computing {
                    keyshares: keyshares.clone(),
                });
            }

            Ok(state)
        })
    }

    pub fn set_pubkey(&mut self, pubkey: Vec<u8>) -> Result<()> {
        self.state.try_mutate(|mut state| {
            let PublicKeyAggregatorState::Computing { keyshares } = &mut state else {
                return Ok(state);
            };

            let keyshares = keyshares.to_owned();

            Ok(PublicKeyAggregatorState::Complete {
                public_key: pubkey,
                keyshares,
            })
        })
    }
}

impl Actor for PublicKeyAggregator {
    type Context = Context<Self>;
}

impl Handler<EnclaveEvent> for PublicKeyAggregator {
    type Result = ();
    fn handle(&mut self, msg: EnclaveEvent, ctx: &mut Self::Context) -> Self::Result {
        match msg {
            EnclaveEvent::KeyshareCreated { data, .. } => ctx.notify(data),
            EnclaveEvent::E3RequestComplete { .. } => ctx.notify(Die),
            _ => (),
        }
    }
}

impl Handler<KeyshareCreated> for PublicKeyAggregator {
    type Result = ResponseActFuture<Self, Result<()>>;

    fn handle(&mut self, event: KeyshareCreated, _: &mut Self::Context) -> Self::Result {
        let address = event.node.clone();
        let e3_id = event.e3_id.clone();
        let pubkey = event.pubkey.clone();

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

                    act.add_keyshare(pubkey)?;

                    if let Some(PublicKeyAggregatorState::Computing { keyshares }) =
                        &act.state.get()
                    {
                        ctx.notify(ComputeAggregate {
                            keyshares: keyshares.clone(),
                            e3_id,
                        })
                    }

                    Ok(())
                }),
        )
    }
}

impl Handler<ComputeAggregate> for PublicKeyAggregator {
    type Result = Result<()>;

    fn handle(&mut self, msg: ComputeAggregate, ctx: &mut Self::Context) -> Self::Result {
        let pubkey = self.fhe.get_aggregate_public_key(GetAggregatePublicKey {
            keyshares: msg.keyshares.clone(),
        })?;

        // Update the local state
        self.set_pubkey(pubkey.clone())?;

        ctx.notify(NotifyNetwork {
            pubkey,
            e3_id: msg.e3_id,
        });
        Ok(())
    }
}

impl Handler<NotifyNetwork> for PublicKeyAggregator {
    type Result = ResponseActFuture<Self, Result<()>>;
    fn handle(&mut self, msg: NotifyNetwork, _: &mut Self::Context) -> Self::Result {
        Box::pin(
            self.sortition
                .send(GetNodesForE3 {
                    e3_id: msg.e3_id.clone(),
                    chain_id: msg.e3_id.chain_id(),
                })
                .into_actor(self)
                .map(move |res, act, _| {
                    let nodes = res?;

                    let pubkey = msg.pubkey.clone();

                    let event = EnclaveEvent::from(PublicKeyAggregated {
                        pubkey,
                        e3_id: msg.e3_id.clone(),
                        nodes: OrderedSet::from(nodes),
                    });
                    act.bus.do_send(event);
                    Ok(())
                }),
        )
    }
}

impl Handler<Die> for PublicKeyAggregator {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop()
    }
}
