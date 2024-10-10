use actix::prelude::*;
use anyhow::Result;
use enclave_core::{
    Die, E3id, EnclaveEvent, EventBus, KeyshareCreated, OrderedSet, PublicKeyAggregated, Seed,
};
use fhe::{Fhe, GetAggregatePublicKey};
use sortition::{GetHasNode, GetNodes, Sortition};
use std::sync::Arc;
use tracing::error;

#[derive(Debug, Clone)]
pub enum PublicKeyAggregatorState {
    Collecting {
        threshold_m: usize,
        keyshares: OrderedSet<Vec<u8>>,
        seed: Seed,
    },
    Computing {
        keyshares: OrderedSet<Vec<u8>>,
    },
    Complete {
        public_key: Vec<u8>,
        keyshares: OrderedSet<Vec<u8>>,
    },
}

#[derive(Message)]
#[rtype(result = "anyhow::Result<()>")]
struct ComputeAggregate {
    pub keyshares: OrderedSet<Vec<u8>>,
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
    bus: Addr<EventBus>,
    sortition: Addr<Sortition>,
    e3_id: E3id,
    state: PublicKeyAggregatorState,
    src_chain_id: u64,
}

/// Aggregate PublicKey for a committee of nodes. This actor listens for KeyshareCreated events
/// around a particular e3_id and aggregates the public key based on this and once done broadcasts
/// a EnclaveEvent::PublicKeyAggregated event on the event bus. Note events are hashed and
/// identical events will not be triggered twice.
/// It is expected to change this mechanism as we work through adversarial scenarios and write tests
/// for them.
impl PublicKeyAggregator {
    pub fn new(
        fhe: Arc<Fhe>,
        bus: Addr<EventBus>,
        sortition: Addr<Sortition>,
        e3_id: E3id,
        threshold_m: usize,
        seed: Seed,
        src_chain_id: u64,
    ) -> Self {
        PublicKeyAggregator {
            fhe,
            bus,
            e3_id,
            sortition,
            src_chain_id,
            state: PublicKeyAggregatorState::Collecting {
                threshold_m,
                keyshares: OrderedSet::new(),
                seed,
            },
        }
    }

    pub fn add_keyshare(&mut self, keyshare: Vec<u8>) -> Result<PublicKeyAggregatorState> {
        let PublicKeyAggregatorState::Collecting {
            threshold_m,
            keyshares,
            ..
        } = &mut self.state
        else {
            return Err(anyhow::anyhow!("Can only add keyshare in Collecting state"));
        };
        keyshares.insert(keyshare);
        if keyshares.len() == *threshold_m {
            return Ok(PublicKeyAggregatorState::Computing {
                keyshares: keyshares.clone(),
            });
        }

        Ok(self.state.clone())
    }

    pub fn set_pubkey(&mut self, pubkey: Vec<u8>) -> Result<PublicKeyAggregatorState> {
        let PublicKeyAggregatorState::Computing { keyshares } = &mut self.state else {
            return Ok(self.state.clone());
        };

        let keyshares = keyshares.to_owned();

        Ok(PublicKeyAggregatorState::Complete {
            public_key: pubkey,
            keyshares,
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
        let PublicKeyAggregatorState::Collecting {
            threshold_m, seed, ..
        } = self.state.clone()
        else {
            error!(state=?self.state, "Aggregator has been closed for collecting keyshares.");
            return Box::pin(fut::ready(Ok(())));
        };

        let size = threshold_m;
        let address = event.node;
        let e3_id = event.e3_id.clone();
        let pubkey = event.pubkey.clone();

        Box::pin(
            self.sortition
                .send(GetHasNode {
                    address,
                    size,
                    seed,
                })
                .into_actor(self)
                .map(move |res, act, ctx| {
                    // NOTE: Returning Ok(()) on errors as we probably dont need a result type here since
                    // we will not be doing a send
                    let has_node = res?;
                    if !has_node {
                        error!("Node not found in committee");
                        return Ok(());
                    }

                    if e3_id != act.e3_id {
                        error!("Wrong e3_id sent to aggregator. This should not happen.");
                        return Ok(());
                    }

                    // add the keyshare and
                    act.state = act.add_keyshare(pubkey)?;

                    // Check the state and if it has changed to the computing
                    if let PublicKeyAggregatorState::Computing { keyshares } = &act.state {
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
        self.state = self.set_pubkey(pubkey.clone())?;
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
                .send(GetNodes)
                .into_actor(self)
                .map(move |res, act, _| {
                    let nodes = res?;

                    let event = EnclaveEvent::from(PublicKeyAggregated {
                        pubkey: msg.pubkey.clone(),
                        e3_id: msg.e3_id.clone(),
                        nodes: OrderedSet::from(nodes),
                        src_chain_id: act.src_chain_id,
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
