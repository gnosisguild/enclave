// SPDX-License-Identifier: LGPL-3.0-only
//
// This file is provided WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY
// or FITNESS FOR A PARTICULAR PURPOSE.

use actix::prelude::*;
use anyhow::Result;
use e3_data::Persistable;
use e3_events::{
    prelude::*, BusHandle, Die, E3id, EnclaveEvent, EnclaveEventData, KeyshareCreated, OrderedSet,
    PublicKeyAggregated, Seed,
};
use e3_fhe::{Fhe, GetAggregatePublicKey};
use e3_utils::ArcBytes;
use std::sync::Arc;
use tracing::{error, info};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum PublicKeyAggregatorState {
    Collecting {
        threshold_n: usize,
        keyshares: OrderedSet<ArcBytes>,
        seed: Seed,
        nodes: OrderedSet<String>,
    },
    Computing {
        keyshares: OrderedSet<ArcBytes>,
        nodes: OrderedSet<String>,
    },
    Complete {
        public_key: Vec<u8>,
        keyshares: OrderedSet<ArcBytes>,
        nodes: OrderedSet<String>,
    },
}

impl PublicKeyAggregatorState {
    pub fn init(threshold_n: usize, seed: Seed) -> Self {
        PublicKeyAggregatorState::Collecting {
            threshold_n,
            keyshares: OrderedSet::new(),
            seed,
            nodes: OrderedSet::new(),
        }
    }
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<()>")]
struct ComputeAggregate {
    pub keyshares: OrderedSet<ArcBytes>,
    pub e3_id: E3id,
}

pub struct PublicKeyAggregator {
    fhe: Arc<Fhe>,
    bus: BusHandle,
    e3_id: E3id,
    state: Persistable<PublicKeyAggregatorState>,
}

pub struct PublicKeyAggregatorParams {
    pub fhe: Arc<Fhe>,
    pub bus: BusHandle,
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
            e3_id: params.e3_id,
            state,
        }
    }

    pub fn add_keyshare(&mut self, keyshare: ArcBytes, node: String) -> Result<()> {
        self.state.try_mutate(|mut state| {
            let PublicKeyAggregatorState::Collecting {
                threshold_n,
                keyshares,
                nodes,
                ..
            } = &mut state
            else {
                return Err(anyhow::anyhow!("Can only add keyshare in Collecting state"));
            };

            keyshares.insert(keyshare);
            nodes.insert(node);
            info!(
                "PublicKeyAggregator got keyshares {}/{}",
                keyshares.len(),
                threshold_n
            );
            if keyshares.len() == *threshold_n {
                info!("Computing aggregate public key...");
                return Ok(PublicKeyAggregatorState::Computing {
                    keyshares: std::mem::take(keyshares),
                    nodes: std::mem::take(nodes),
                });
            }

            Ok(state)
        })
    }

    pub fn set_pubkey(&mut self, pubkey: Vec<u8>) -> Result<()> {
        self.state.try_mutate(|mut state| {
            let PublicKeyAggregatorState::Computing { keyshares, nodes } = &mut state else {
                return Ok(state);
            };

            Ok(PublicKeyAggregatorState::Complete {
                public_key: pubkey,
                keyshares: std::mem::take(keyshares),
                nodes: std::mem::take(nodes),
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
        match msg.into_data() {
            EnclaveEventData::KeyshareCreated(data) => ctx.notify(data),
            EnclaveEventData::E3RequestComplete(_) => ctx.notify(Die),
            _ => (),
        }
    }
}

impl Handler<KeyshareCreated> for PublicKeyAggregator {
    type Result = Result<()>;

    fn handle(&mut self, event: KeyshareCreated, ctx: &mut Self::Context) -> Self::Result {
        let e3_id = event.e3_id.clone();
        let pubkey = event.pubkey.clone();
        let node = event.node.clone();

        if e3_id != self.e3_id {
            error!("Wrong e3_id sent to aggregator. This should not happen.");
            return Ok(());
        }

        self.add_keyshare(pubkey, node)?;

        if let Some(PublicKeyAggregatorState::Computing { keyshares, .. }) = &self.state.get() {
            ctx.notify(ComputeAggregate {
                keyshares: keyshares.clone(),
                e3_id,
            })
        }

        Ok(())
    }
}

impl Handler<ComputeAggregate> for PublicKeyAggregator {
    type Result = Result<()>;

    fn handle(&mut self, msg: ComputeAggregate, _: &mut Self::Context) -> Self::Result {
        info!("Computing Aggregate PublicKey...");
        let pubkey = self.fhe.get_aggregate_public_key(GetAggregatePublicKey {
            keyshares: msg.keyshares,
        })?;

        // Update the local state
        self.set_pubkey(pubkey)?;

        if let Some(PublicKeyAggregatorState::Complete {
            public_key: pubkey,
            nodes,
            ..
        }) = self.state.get()
        {
            info!("Notifying network of PublicKey");
            info!("Sending PublicKeyAggregated...");
            let event = PublicKeyAggregated {
                pubkey,
                e3_id: msg.e3_id,
                nodes,
            };
            self.bus.publish(event)?;
        }
        Ok(())
    }
}

impl Handler<Die> for PublicKeyAggregator {
    type Result = ();
    fn handle(&mut self, _: Die, ctx: &mut Self::Context) -> Self::Result {
        ctx.stop()
    }
}
